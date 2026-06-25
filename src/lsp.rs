//! The page side of the language-server bridge: the LSP client. It connects to
//! the desktop relay, runs the rust-analyzer handshake, syncs open Rust files
//! with `didOpen`/`didChange`, and turns `publishDiagnostics` into the editor's
//! diagnostics strip. It also requests completion at the caret and hover under
//! the pointer, anchoring both popups. Starting the server is gated behind a
//! consent toast, since it spawns a process.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use leptos::prelude::*;
use protocol::{Diagnostic, LspServerMessage, SearchHit, Severity};
use serde_json::{Value, json};
use web_sys::WebSocket;

use crate::state::{
    CompletionEntry, CompletionMenu, EditorState, HoverCard, PluginKind, SidebarView, basename,
    language_for_path,
};

mod edits;
mod requests;
mod text;
mod transport;

pub use edits::apply_pending_edits;
use edits::{RangeEdit, apply_edits_to_file, apply_workspace_edit, parse_edits};
use requests::run_code_action;
pub use requests::{
    accept_completion, apply_code_action, format_and_save, format_document, goto_diagnostic,
    request_code_actions, request_completion, request_hover_at, request_hover_at_caret,
    request_locations, request_references, request_signature_help, request_symbols,
    request_workspace_symbols, start_rename, submit_rename,
};
use transport::{connect, file_uri, notify, path_from_uri, send_raw, send_request};

enum Pending {
    Completion { prefix: String, x: f64, y: f64 },
    Hover { x: f64, y: f64 },
    Signature { x: f64, y: f64 },
    Definition,
    Format { path: String },
    References,
    Symbols { path: String },
    Rename,
    CodeActions,
    ResolveAction,
    Command,
}

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
    suppress_completion: bool,
    rename_position: Option<(String, u32, u32)>,
    code_actions: Vec<Value>,
    pending_edits: HashMap<String, Vec<RangeEdit>>,
    raw_diagnostics: HashMap<String, Vec<Value>>,
    save_after_format: HashSet<String>,
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
            suppress_completion: false,
            rename_position: None,
            code_actions: Vec::new(),
            pending_edits: HashMap::new(),
            raw_diagnostics: HashMap::new(),
            save_after_format: HashSet::new(),
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

/// The most requests left in flight at once before the oldest is dropped, a
/// backstop so a request the server never answers cannot leak its entry.
const MAX_PENDING: usize = 64;

/// Records an in-flight request awaiting its reply. Supersedes any earlier
/// request of the same kind, so when a stale reply arrives its id is gone and
/// the outdated result never applies, and caps the map against a lost reply.
fn track(id: i64, pending: Pending) {
    client(|client| {
        let kind = std::mem::discriminant(&pending);
        client
            .pending
            .retain(|_, existing| std::mem::discriminant(existing) != kind);
        client.pending.insert(id, pending);
        while client.pending.len() > MAX_PENDING {
            let Some(oldest) = client.pending.keys().copied().min() else {
                break;
            };
            client.pending.remove(&oldest);
        }
    });
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
    let detail = item
        .get("detail")
        .and_then(Value::as_str)
        .or_else(|| item.pointer("/labelDetails/detail").and_then(Value::as_str))
        .unwrap_or_default()
        .to_string();
    let kind = completion_kind(item.get("kind").and_then(Value::as_u64)).to_string();
    Some(CompletionEntry {
        label,
        insert,
        detail,
        kind,
    })
}

fn completion_kind(kind: Option<u64>) -> &'static str {
    match kind {
        Some(2) => "method",
        Some(3) => "fn",
        Some(4) => "ctor",
        Some(5) => "field",
        Some(6) => "var",
        Some(7) => "class",
        Some(8) => "trait",
        Some(9) => "mod",
        Some(10) => "prop",
        Some(11) => "unit",
        Some(12) => "value",
        Some(13) => "enum",
        Some(14) => "kw",
        Some(15) => "snip",
        Some(20) => "variant",
        Some(21) => "const",
        Some(22) => "struct",
        Some(23) => "event",
        Some(24) => "op",
        Some(25) => "type",
        _ => "",
    }
}

fn apply_locations(state: EditorState, value: &Value) {
    let result = value.get("result");
    let raw = match result {
        Some(Value::Array(items)) => items.iter().collect::<Vec<_>>(),
        Some(object) if object.is_object() => vec![object],
        _ => Vec::new(),
    };
    let locations: Vec<(String, u32)> = raw
        .iter()
        .filter_map(|location| {
            let uri = location
                .get("uri")
                .or_else(|| location.get("targetUri"))
                .and_then(Value::as_str)?;
            let line = location
                .pointer("/range/start/line")
                .or_else(|| location.pointer("/targetSelectionRange/start/line"))
                .and_then(Value::as_u64)?;
            Some((path_from_uri(uri), line as u32 + 1))
        })
        .collect();
    match locations.as_slice() {
        [] => {}
        [(path, line)] => {
            crate::fs::read_file(path);
            state.goto.set(Some((path.clone(), *line)));
        }
        _ => {
            let hits = locations
                .into_iter()
                .map(|(path, line)| SearchHit {
                    text: format!("{}:{}", basename(&path), line),
                    path,
                    line,
                })
                .collect();
            state.symbol_picker.set(hits);
        }
    }
}

fn apply_format(state: EditorState, value: &Value, path: &str) {
    if let Some(raw) = value.get("result").and_then(Value::as_array) {
        apply_edits_to_file(state, path, &parse_edits(raw));
    }
    if client(|client| client.save_after_format.remove(path)) {
        let text = state.buffer_source(PluginKind::File, &Some(path.to_string()));
        crate::fs::write_file(path, text);
    }
}

fn apply_references(state: EditorState, value: &Value) {
    let Some(items) = value.get("result").and_then(Value::as_array) else {
        return;
    };
    let hits: Vec<SearchHit> = items
        .iter()
        .filter_map(|item| {
            let uri = item.get("uri").and_then(Value::as_str)?;
            let line = item.pointer("/range/start/line").and_then(Value::as_u64)?;
            let path = path_from_uri(uri);
            let text = basename(&path).to_string();
            Some(SearchHit {
                path,
                line: line as u32 + 1,
                text,
            })
        })
        .collect();
    state.search_results.set(hits);
    state.sidebar_view.set(SidebarView::Search);
}

fn apply_symbols(state: EditorState, value: &Value, path: &str) {
    let Some(items) = value.get("result").and_then(Value::as_array) else {
        return;
    };
    let mut hits = Vec::new();
    collect_symbols(items, path, &mut hits);
    state.symbol_picker.set(hits);
}

fn collect_symbols(items: &[Value], path: &str, out: &mut Vec<SearchHit>) {
    for item in items {
        let name = item
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let line = item
            .pointer("/selectionRange/start/line")
            .or_else(|| item.pointer("/range/start/line"))
            .or_else(|| item.pointer("/location/range/start/line"))
            .and_then(Value::as_u64);
        if let Some(line) = line {
            let symbol_path = item
                .pointer("/location/uri")
                .and_then(Value::as_str)
                .map(path_from_uri)
                .unwrap_or_else(|| path.to_string());
            out.push(SearchHit {
                path: symbol_path,
                line: line as u32 + 1,
                text: name,
            });
        }
        if let Some(children) = item.get("children").and_then(Value::as_array) {
            collect_symbols(children, path, out);
        }
    }
}

fn apply_code_actions(state: EditorState, value: &Value) {
    let Some(items) = value.get("result").and_then(Value::as_array) else {
        return;
    };
    let actions: Vec<Value> = items.to_vec();
    let titles: Vec<String> = actions
        .iter()
        .filter_map(|action| {
            action
                .get("title")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect();
    client(|client| client.code_actions = actions);
    state.code_actions.set(titles);
}

fn apply_signature(state: EditorState, value: &Value, x: f64, y: f64) {
    let result = value.get("result");
    let signatures = result
        .and_then(|result| result.get("signatures"))
        .and_then(Value::as_array);
    let Some(signatures) = signatures.filter(|items| !items.is_empty()) else {
        state.hover.set(None);
        return;
    };
    let active = result
        .and_then(|result| result.get("activeSignature"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let label = signatures
        .get(active)
        .or_else(|| signatures.first())
        .and_then(|signature| signature.get("label"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if label.trim().is_empty() {
        state.hover.set(None);
    } else {
        state.hover.set(Some(HoverCard {
            text: label.to_string(),
            x,
            y,
        }));
    }
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

fn handle(state: EditorState, message: LspServerMessage) {
    match message {
        LspServerMessage::Started => {
            client(|client| {
                client.ready = false;
                client.versions.clear();
                client.pending.clear();
            });
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
    match value.get("method").and_then(Value::as_str) {
        Some("textDocument/publishDiagnostics") => {
            if let Some(params) = value.get("params") {
                apply_diagnostics(state, params);
            }
            return;
        }
        Some("workspace/applyEdit") => {
            if let Some(edit) = value.pointer("/params/edit") {
                apply_workspace_edit(state, edit);
            }
            if let Some(id) = value.get("id") {
                send_raw(json!({ "jsonrpc": "2.0", "id": id, "result": { "applied": true } }));
            }
            return;
        }
        _ => {}
    }
    let Some(id) = value.get("id").and_then(Value::as_i64) else {
        return;
    };
    if let Some(pending) = client(|client| client.pending.remove(&id)) {
        match pending {
            Pending::Completion { prefix, x, y } => apply_completion(state, &value, prefix, x, y),
            Pending::Hover { x, y } => apply_hover(state, &value, x, y),
            Pending::Signature { x, y } => apply_signature(state, &value, x, y),
            Pending::Definition => apply_locations(state, &value),
            Pending::Format { path } => apply_format(state, &value, &path),
            Pending::References => apply_references(state, &value),
            Pending::Symbols { path } => apply_symbols(state, &value, &path),
            Pending::Rename => {
                if let Some(result) = value.get("result") {
                    apply_workspace_edit(state, result);
                }
            }
            Pending::CodeActions => apply_code_actions(state, &value),
            Pending::ResolveAction => {
                if let Some(result) = value.get("result").filter(|result| result.is_object()) {
                    run_code_action(state, result.clone());
                }
            }
            Pending::Command => {
                if let Some(result) = value.get("result").filter(|result| result.is_object()) {
                    apply_workspace_edit(state, result);
                }
            }
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
    let raw = params
        .get("diagnostics")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let diagnostics: Vec<Diagnostic> = raw.iter().map(to_diagnostic).collect();
    client(|client| {
        client.diagnostics.insert(path.clone(), diagnostics);
        client.raw_diagnostics.insert(path.clone(), raw);
    });
    let problems = client(|client| {
        let mut problems: Vec<(String, Diagnostic)> = client
            .diagnostics
            .iter()
            .flat_map(|(path, items)| {
                items
                    .iter()
                    .map(|diagnostic| (path.clone(), diagnostic.clone()))
            })
            .collect();
        problems.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.line.cmp(&right.1.line)));
        problems
    });
    state.problems.set(problems);
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
