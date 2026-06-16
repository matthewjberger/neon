//! The page side of the language-server bridge: the LSP client. It connects to
//! the desktop relay, runs the rust-analyzer handshake, syncs open Rust files
//! with `didOpen`/`didChange`, and turns `publishDiagnostics` into the editor's
//! diagnostics strip. It also requests completion at the caret and hover under
//! the pointer, anchoring both popups. Starting the server is gated behind a
//! consent toast, since it spawns a process.

use std::cell::RefCell;
use std::collections::HashMap;

use leptos::prelude::*;
use protocol::{Diagnostic, LspClientMessage, LspServerMessage, Severity};
use serde_json::{Value, json};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlTextAreaElement, MessageEvent, WebSocket};

use crate::state::{
    CompletionEntry, CompletionMenu, EditorState, HoverCard, PluginKind, language_for_path,
};

enum Pending {
    Completion { prefix: String, x: f64, y: f64 },
    Hover { x: f64, y: f64 },
}

const LSP_URL: &str = "ws://127.0.0.1:8793";
const RECONNECT_MS: i32 = 1000;

/// The LSP client's per-page state, in one place: the socket, the handshake
/// flag, the request-id counter, the per-file document versions, the latest
/// diagnostics, and the in-flight requests awaiting a reply.
struct Client {
    socket: Option<WebSocket>,
    ready: bool,
    next_id: i64,
    versions: HashMap<String, i64>,
    diagnostics: HashMap<String, Vec<Diagnostic>>,
    pending: HashMap<i64, Pending>,
}

impl Client {
    fn new() -> Self {
        Self {
            socket: None,
            ready: false,
            next_id: 2,
            versions: HashMap::new(),
            diagnostics: HashMap::new(),
            pending: HashMap::new(),
        }
    }
}

thread_local! {
    static CLIENT: RefCell<Client> = RefCell::new(Client::new());
}

fn client<R>(action: impl FnOnce(&mut Client) -> R) -> R {
    CLIENT.with(|client| action(&mut client.borrow_mut()))
}

fn ready() -> bool {
    client(|client| client.ready)
}

fn next_id() -> i64 {
    client(|client| {
        let id = client.next_id;
        client.next_id += 1;
        id
    })
}

/// Shows the consent toast for a Rust file, unless the server is already running.
pub fn did_open(state: EditorState, path: &str) {
    if language_for_path(path) != "rust" {
        return;
    }
    if state.lsp_started.get_untracked() {
        open_document(state, path);
    } else {
        state.lsp_consent.set(true);
    }
}

/// Accepts the consent toast: enables the bridge and starts the handshake.
pub fn enable(state: EditorState) {
    state.lsp_consent.set(false);
    if state.lsp_started.get_untracked() {
        return;
    }
    state.lsp_started.set(true);
    crate::ipc::notify_host("enable-lsp");
    connect(state);
}

/// Sends a full-text `didChange` for a file the server already has open.
pub fn did_change(state: EditorState, path: &str) {
    if !ready() {
        return;
    }
    let open = client(|client| client.versions.contains_key(path));
    if !open {
        return;
    }
    let text = state.buffer_source(PluginKind::File, &Some(path.to_string()));
    let version = client(|client| {
        let entry = client.versions.entry(path.to_string()).or_insert(0);
        *entry += 1;
        *entry
    });
    notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": file_uri(path), "version": version },
            "contentChanges": [{ "text": text }],
        }),
    );
}

/// Sets the diagnostics strip from the focused buffer: a file's stored LSP
/// diagnostics, or empty for a plugin (the language worker repopulates those).
pub fn refresh_diagnostics(state: EditorState) {
    let buffer = state.focused_buffer();
    if buffer.kind == PluginKind::File
        && let Some(path) = buffer.id
    {
        let stored = client(|client| client.diagnostics.get(&path).cloned().unwrap_or_default());
        state.diagnostics.set(stored);
    } else {
        state.diagnostics.set(Vec::new());
    }
}

/// Requests completion at the caret of the focused Rust file.
pub fn request_completion(state: EditorState) {
    if !ready() {
        return;
    }
    let buffer = state.focused_buffer();
    if buffer.kind != PluginKind::File {
        return;
    }
    let Some(path) = buffer.id else {
        return;
    };
    if language_for_path(&path) != "rust" {
        return;
    }
    let Some(element) = crate::components::find::active() else {
        return;
    };
    let value = element.value();
    let caret = element.selection_start().ok().flatten().unwrap_or(0);
    let (line, character) = line_character(&value, caret);
    let prefix = word_prefix(&value, caret);
    let (x, y) = caret_pixel(&element, line, character);
    let id = next_id();
    client(|client| {
        client
            .pending
            .insert(id, Pending::Completion { prefix, x, y });
    });
    send_request_id(
        id,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": file_uri(&path) },
            "position": { "line": line, "character": character },
        }),
    );
}

/// Requests hover for the document position under a client pixel point.
pub fn request_hover_at(state: EditorState, client_x: f64, client_y: f64) {
    if !ready() {
        return;
    }
    let buffer = state.focused_buffer();
    if buffer.kind != PluginKind::File {
        return;
    }
    let Some(path) = buffer.id else {
        return;
    };
    if language_for_path(&path) != "rust" {
        return;
    }
    let Some(element) = crate::components::find::active() else {
        return;
    };
    let rect = element.get_bounding_client_rect();
    let (pad_left, pad_top, char_width, line_height) = metrics(&element);
    let column = (((client_x - rect.left() - pad_left + element.scroll_left() as f64) / char_width)
        .floor())
    .max(0.0) as u32;
    let line = (((client_y - rect.top() - pad_top + element.scroll_top() as f64) / line_height)
        .floor())
    .max(0.0) as u32;
    let id = next_id();
    client(|client| {
        client.pending.insert(
            id,
            Pending::Hover {
                x: client_x,
                y: client_y,
            },
        );
    });
    send_request_id(
        id,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": file_uri(&path) },
            "position": { "line": line, "character": column },
        }),
    );
}

/// Accepts a completion candidate, replacing the typed prefix.
pub fn accept_completion(state: EditorState, index: usize) {
    let Some(menu) = state.completion.get_untracked() else {
        return;
    };
    let Some(entry) = menu.items.get(index) else {
        return;
    };
    let Some(element) = crate::components::find::active() else {
        return;
    };
    let value = element.value();
    let caret = element.selection_start().ok().flatten().unwrap_or(0);
    let prefix_units = menu.prefix.encode_utf16().count() as u32;
    let start = caret.saturating_sub(prefix_units);
    let replaced = splice_utf16(&value, start, caret, &entry.insert);
    element.set_value(&replaced);
    let new_caret = start + entry.insert.encode_utf16().count() as u32;
    let _ = element.set_selection_range(new_caret, new_caret);
    let _ = element.focus();
    if let Ok(event) = web_sys::Event::new("input") {
        let _ = element.dispatch_event(&event);
    }
    state.completion.set(None);
}

fn apply_completion(state: EditorState, value: &Value, prefix: String, x: f64, y: f64) {
    let result = value.get("result");
    let array = result
        .and_then(|result| result.get("items").or(Some(result)))
        .and_then(Value::as_array);
    let items: Vec<CompletionEntry> = array
        .map(|items| items.iter().take(60).filter_map(to_entry).collect())
        .unwrap_or_default();
    if items.is_empty() {
        state.completion.set(None);
        return;
    }
    state.completion.set(Some(CompletionMenu {
        items,
        x,
        y,
        prefix,
    }));
    state.completion_index.set(0);
}

fn to_entry(item: &Value) -> Option<CompletionEntry> {
    let label = item.get("label").and_then(Value::as_str)?.to_string();
    let insert = item
        .get("insertText")
        .and_then(Value::as_str)
        .or_else(|| item.pointer("/textEdit/newText").and_then(Value::as_str))
        .unwrap_or(&label)
        .to_string();
    Some(CompletionEntry { label, insert })
}

fn apply_hover(state: EditorState, value: &Value, x: f64, y: f64) {
    let contents = value.pointer("/result/contents");
    let text = match contents {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Object(map)) => map
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| {
                item.as_str().map(str::to_string).or_else(|| {
                    item.get("value")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };
    if text.trim().is_empty() {
        state.hover.set(None);
    } else {
        state.hover.set(Some(HoverCard { text, x, y }));
    }
}

fn line_character(value: &str, caret: u32) -> (u32, u32) {
    let mut line = 0;
    let mut column = 0;
    let mut seen = 0;
    for character in value.chars() {
        if seen >= caret {
            break;
        }
        let width = character.len_utf16() as u32;
        if character == '\n' {
            line += 1;
            column = 0;
        } else {
            column += width;
        }
        seen += width;
    }
    (line, column)
}

fn word_prefix(value: &str, caret: u32) -> String {
    let mut seen = 0;
    let mut word = String::new();
    for character in value.chars() {
        if seen >= caret {
            break;
        }
        if character.is_alphanumeric() || character == '_' {
            word.push(character);
        } else {
            word.clear();
        }
        seen += character.len_utf16() as u32;
    }
    word
}

fn caret_pixel(element: &HtmlTextAreaElement, line: u32, column: u32) -> (f64, f64) {
    let rect = element.get_bounding_client_rect();
    let (pad_left, pad_top, char_width, line_height) = metrics(element);
    let x = rect.left() + pad_left + column as f64 * char_width - element.scroll_left() as f64;
    let y = rect.top() + pad_top + (line as f64 + 1.0) * line_height - element.scroll_top() as f64;
    (x, y)
}

fn metrics(element: &HtmlTextAreaElement) -> (f64, f64, f64, f64) {
    let style =
        web_sys::window().and_then(|window| window.get_computed_style(element).ok().flatten());
    let font_size = parse_px(style.as_ref(), "font-size").unwrap_or(13.0);
    let line_height = parse_px(style.as_ref(), "line-height").unwrap_or(font_size * 1.5);
    let pad_left = parse_px(style.as_ref(), "padding-left").unwrap_or(0.0);
    let pad_top = parse_px(style.as_ref(), "padding-top").unwrap_or(0.0);
    (pad_left, pad_top, font_size * 0.6, line_height)
}

fn parse_px(style: Option<&web_sys::CssStyleDeclaration>, property: &str) -> Option<f64> {
    let raw = style?.get_property_value(property).ok()?;
    raw.trim_end_matches("px").trim().parse().ok()
}

fn splice_utf16(value: &str, start: u32, end: u32, replacement: &str) -> String {
    let units: Vec<u16> = value.encode_utf16().collect();
    let head = String::from_utf16_lossy(&units[..start as usize]);
    let tail = String::from_utf16_lossy(&units[end as usize..]);
    format!("{head}{replacement}{tail}")
}

fn send_request_id(id: i64, method: &str, params: Value) {
    send_raw(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
}

fn connect(state: EditorState) {
    let Ok(websocket) = WebSocket::new(LSP_URL) else {
        schedule_reconnect(state);
        return;
    };
    let open_state = state;
    let onopen = Closure::<dyn FnMut()>::new(move || {
        if let Some(root) = open_state.workspace_root.get_untracked() {
            send(&LspClientMessage::Start {
                root_uri: file_uri(&root),
            });
        }
    });
    websocket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        if let Some(text) = event.data().as_string()
            && let Ok(message) = serde_json::from_str::<LspServerMessage>(&text)
        {
            handle(state, message);
        }
    });
    websocket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let onclose = Closure::<dyn FnMut()>::new(move || {
        client(|client| client.ready = false);
        schedule_reconnect(state);
    });
    websocket.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    client(|client| client.socket = Some(websocket));
}

fn schedule_reconnect(state: EditorState) {
    client(|client| client.socket = None);
    if !state.lsp_started.get_untracked() {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };
    let callback = Closure::<dyn FnMut()>::new(move || connect(state));
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        RECONNECT_MS,
    );
    callback.forget();
}

fn handle(state: EditorState, message: LspServerMessage) {
    match message {
        LspServerMessage::Started => {
            send_request(
                "initialize",
                json!({
                    "processId": Value::Null,
                    "rootUri": state.workspace_root.get_untracked().map(|root| file_uri(&root)),
                    "capabilities": {
                        "textDocument": {
                            "synchronization": { "didSave": false },
                            "publishDiagnostics": {},
                        }
                    },
                }),
            );
        }
        LspServerMessage::Rpc { json } => {
            if let Ok(value) = serde_json::from_str::<Value>(&json) {
                handle_rpc(state, value);
            }
        }
        LspServerMessage::Log { line } => log(state, line),
        LspServerMessage::Error { message } => log(state, format!("error: {message}")),
        LspServerMessage::Exited { code } => {
            client(|client| client.ready = false);
            log(state, format!("rust-analyzer exited ({code:?})"));
        }
    }
}

fn handle_rpc(state: EditorState, value: Value) {
    if value.get("method").and_then(Value::as_str) == Some("textDocument/publishDiagnostics") {
        if let Some(params) = value.get("params") {
            apply_diagnostics(state, params);
        }
        return;
    }
    let Some(id) = value.get("id").and_then(Value::as_i64) else {
        return;
    };
    if let Some(pending) = client(|client| client.pending.remove(&id)) {
        match pending {
            Pending::Completion { prefix, x, y } => apply_completion(state, &value, prefix, x, y),
            Pending::Hover { x, y } => apply_hover(state, &value, x, y),
        }
        return;
    }
    if id == 1 && value.get("result").is_some() {
        notify("initialized", json!({}));
        client(|client| client.ready = true);
        for path in open_rust_files(state) {
            open_document(state, &path);
        }
    }
}

fn open_document(state: EditorState, path: &str) {
    let already = client(|client| client.versions.contains_key(path));
    if already {
        return;
    }
    client(|client| {
        client.versions.insert(path.to_string(), 0);
    });
    let text = state.buffer_source(PluginKind::File, &Some(path.to_string()));
    notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": file_uri(path),
                "languageId": "rust",
                "version": 0,
                "text": text,
            }
        }),
    );
}

fn apply_diagnostics(state: EditorState, params: &Value) {
    let Some(uri) = params.get("uri").and_then(Value::as_str) else {
        return;
    };
    let path = path_from_uri(uri);
    let diagnostics: Vec<Diagnostic> = params
        .get("diagnostics")
        .and_then(Value::as_array)
        .map(|items| items.iter().map(to_diagnostic).collect())
        .unwrap_or_default();
    client(|client| {
        client.diagnostics.insert(path.clone(), diagnostics);
    });
    let focused = state.focused_buffer();
    if focused.kind == PluginKind::File && focused.id.as_deref() == Some(path.as_str()) {
        refresh_diagnostics(state);
    }
}

fn to_diagnostic(value: &Value) -> Diagnostic {
    let line = value
        .pointer("/range/start/line")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32
        + 1;
    let column = value
        .pointer("/range/start/character")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32
        + 1;
    let message = value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let severity = match value.get("severity").and_then(Value::as_u64) {
        Some(1) => Severity::Error,
        _ => Severity::Warning,
    };
    Diagnostic {
        message,
        line,
        column,
        severity,
    }
}

fn open_rust_files(state: EditorState) -> Vec<String> {
    state.files.with_untracked(|files| {
        files
            .iter()
            .filter(|file| language_for_path(&file.path) == "rust")
            .map(|file| file.path.clone())
            .collect()
    })
}

fn log(state: EditorState, line: String) {
    state.lsp_log.update(|entries| {
        entries.push(line);
        let overflow = entries.len().saturating_sub(500);
        if overflow > 0 {
            entries.drain(0..overflow);
        }
    });
}

fn send_request(method: &str, params: Value) {
    send_raw(json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }));
}

fn notify(method: &str, params: Value) {
    send_raw(json!({ "jsonrpc": "2.0", "method": method, "params": params }));
}

fn send_raw(message: Value) {
    send(&LspClientMessage::Rpc {
        json: message.to_string(),
    });
}

fn send(message: &LspClientMessage) {
    client(|client| {
        if let Some(websocket) = client.socket.as_ref()
            && websocket.ready_state() == WebSocket::OPEN
            && let Ok(text) = serde_json::to_string(message)
        {
            let _ = websocket.send_with_str(&text);
        }
    });
}

fn file_uri(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    if normalized.starts_with('/') {
        format!("file://{normalized}")
    } else {
        format!("file:///{normalized}")
    }
}

fn path_from_uri(uri: &str) -> String {
    let trimmed = uri
        .strip_prefix("file:///")
        .or_else(|| uri.strip_prefix("file://"))
        .unwrap_or(uri);
    trimmed.replace('/', "\\")
}
