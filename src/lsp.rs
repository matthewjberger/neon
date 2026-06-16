//! The page side of the language-server bridge: the LSP client. It connects to
//! the desktop relay, runs the rust-analyzer handshake, syncs open Rust files
//! with `didOpen`/`didChange`, and turns `publishDiagnostics` into the editor's
//! diagnostics strip. Completion and hover ride the same channel later. Starting
//! the server is gated behind a consent toast, since it spawns a process.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use leptos::prelude::*;
use protocol::{Diagnostic, LspClientMessage, LspServerMessage, Severity};
use serde_json::{Value, json};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

use crate::state::{EditorState, PluginKind, language_for_path};

const LSP_URL: &str = "ws://127.0.0.1:8793";
const RECONNECT_MS: i32 = 1000;

thread_local! {
    static SOCKET: RefCell<Option<WebSocket>> = const { RefCell::new(None) };
    static VERSIONS: RefCell<HashMap<String, i64>> = RefCell::new(HashMap::new());
    static DIAGNOSTICS: RefCell<HashMap<String, Vec<Diagnostic>>> = RefCell::new(HashMap::new());
    static READY: Cell<bool> = const { Cell::new(false) };
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
    if !READY.with(Cell::get) {
        return;
    }
    let open = VERSIONS.with(|versions| versions.borrow().contains_key(path));
    if !open {
        return;
    }
    let text = state.buffer_source(PluginKind::File, &Some(path.to_string()));
    let version = VERSIONS.with(|versions| {
        let mut versions = versions.borrow_mut();
        let entry = versions.entry(path.to_string()).or_insert(0);
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
        let stored = DIAGNOSTICS.with(|map| map.borrow().get(&path).cloned().unwrap_or_default());
        state.diagnostics.set(stored);
    } else {
        state.diagnostics.set(Vec::new());
    }
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
        READY.with(|ready| ready.set(false));
        schedule_reconnect(state);
    });
    websocket.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    SOCKET.with(|slot| *slot.borrow_mut() = Some(websocket));
}

fn schedule_reconnect(state: EditorState) {
    SOCKET.with(|slot| *slot.borrow_mut() = None);
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
            READY.with(|ready| ready.set(false));
            log(state, format!("rust-analyzer exited ({code:?})"));
        }
    }
}

fn handle_rpc(state: EditorState, value: Value) {
    let method = value.get("method").and_then(Value::as_str);
    if method == Some("textDocument/publishDiagnostics") {
        if let Some(params) = value.get("params") {
            apply_diagnostics(state, params);
        }
        return;
    }
    let is_initialize = value
        .get("id")
        .and_then(Value::as_i64)
        .map(|id| id == 1)
        .unwrap_or(false);
    if is_initialize && value.get("result").is_some() {
        notify("initialized", json!({}));
        READY.with(|ready| ready.set(true));
        for path in open_rust_files(state) {
            open_document(state, &path);
        }
    }
}

fn open_document(state: EditorState, path: &str) {
    let already = VERSIONS.with(|versions| versions.borrow().contains_key(path));
    if already {
        return;
    }
    VERSIONS.with(|versions| {
        versions.borrow_mut().insert(path.to_string(), 0);
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
    DIAGNOSTICS.with(|map| {
        map.borrow_mut().insert(path.clone(), diagnostics);
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
    SOCKET.with(|slot| {
        if let Some(websocket) = slot.borrow().as_ref()
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
