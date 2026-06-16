//! The page side of the language-server bridge: the LSP client. It connects to
//! the desktop relay, runs the rust-analyzer handshake, syncs open Rust files
//! with `didOpen`/`didChange`, and turns `publishDiagnostics` into the editor's
//! diagnostics strip. It also requests completion at the caret and hover under
//! the pointer, anchoring both popups. Starting the server is gated behind a
//! consent toast, since it spawns a process.

use std::cell::RefCell;
use std::collections::HashMap;

use leptos::prelude::*;
use protocol::{Diagnostic, LspServerMessage, SearchHit, Severity};
use serde_json::{Value, json};
use web_sys::{HtmlTextAreaElement, WebSocket};

use crate::state::{
    CompletionEntry, CompletionMenu, EditorState, HoverCard, PluginKind, SidebarView, basename,
    language_for_path,
};

mod edits;
mod transport;

pub use edits::apply_pending_edits;
use edits::{RangeEdit, apply_edits_to_file, apply_workspace_edit, parse_edits, splice_utf16};
use transport::{
    connect, file_uri, notify, path_from_uri, send_raw, send_request, send_request_id,
};

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
    if client(|client| std::mem::take(&mut client.suppress_completion)) {
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

/// Requests definitions, type definitions, or implementations of the symbol at
/// the caret by method, jumping to the one result or listing many in the picker.
pub fn request_locations(state: EditorState, method: &str) {
    let Some((path, line, character)) = caret_position(state) else {
        return;
    };
    let id = next_id();
    client(|client| {
        client.pending.insert(id, Pending::Definition);
    });
    send_request_id(
        id,
        method,
        json!({
            "textDocument": { "uri": file_uri(&path) },
            "position": { "line": line, "character": character },
        }),
    );
}

/// Requests a whole-document format from the server and applies the edits.
pub fn format_document(state: EditorState) {
    let Some((path, _, _)) = caret_position(state) else {
        return;
    };
    let id = next_id();
    client(|client| {
        client
            .pending
            .insert(id, Pending::Format { path: path.clone() });
    });
    send_request_id(
        id,
        "textDocument/formatting",
        json!({
            "textDocument": { "uri": file_uri(&path) },
            "options": { "tabSize": 4, "insertSpaces": true },
        }),
    );
}

/// Requests hover for the symbol at the caret, anchored under it.
pub fn request_hover_at_caret(state: EditorState) {
    let Some((path, line, character)) = caret_position(state) else {
        return;
    };
    let Some(element) = crate::components::find::active() else {
        return;
    };
    let (x, y) = caret_pixel(&element, line, character);
    let id = next_id();
    client(|client| {
        client.pending.insert(id, Pending::Hover { x, y });
    });
    send_request_id(
        id,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": file_uri(&path) },
            "position": { "line": line, "character": character },
        }),
    );
}

/// Requests signature help at the caret, shown in the hover card.
pub fn request_signature_help(state: EditorState) {
    let Some((path, line, character)) = caret_position(state) else {
        return;
    };
    let Some(element) = crate::components::find::active() else {
        return;
    };
    let (x, y) = caret_pixel(&element, line, character);
    let id = next_id();
    client(|client| {
        client.pending.insert(id, Pending::Signature { x, y });
    });
    send_request_id(
        id,
        "textDocument/signatureHelp",
        json!({
            "textDocument": { "uri": file_uri(&path) },
            "position": { "line": line, "character": character },
        }),
    );
}

/// Requests all references to the symbol at the caret into the search panel.
pub fn request_references(state: EditorState) {
    let Some((path, line, character)) = caret_position(state) else {
        return;
    };
    let id = next_id();
    client(|client| {
        client.pending.insert(id, Pending::References);
    });
    send_request_id(
        id,
        "textDocument/references",
        json!({
            "textDocument": { "uri": file_uri(&path) },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": true },
        }),
    );
}

/// Searches workspace symbols matching the word at the caret into the picker.
pub fn request_workspace_symbols() {
    if !ready() {
        return;
    }
    let query = crate::components::find::active()
        .map(|element| {
            let value = element.value();
            let caret = element.selection_start().ok().flatten().unwrap_or(0);
            word_at(&value, caret)
        })
        .unwrap_or_default();
    let id = next_id();
    client(|client| {
        client.pending.insert(
            id,
            Pending::Symbols {
                path: String::new(),
            },
        );
    });
    send_request_id(id, "workspace/symbol", json!({ "query": query }));
}

/// Requests the document symbols of the focused file into the search panel.
pub fn request_symbols(state: EditorState) {
    let Some((path, _, _)) = caret_position(state) else {
        return;
    };
    let id = next_id();
    client(|client| {
        client
            .pending
            .insert(id, Pending::Symbols { path: path.clone() });
    });
    send_request_id(
        id,
        "textDocument/documentSymbol",
        json!({ "textDocument": { "uri": file_uri(&path) } }),
    );
}

/// Opens the rename prompt for the symbol at the caret.
pub fn start_rename(state: EditorState) {
    let Some((path, line, character)) = caret_position(state) else {
        return;
    };
    let Some(element) = crate::components::find::active() else {
        return;
    };
    let value = element.value();
    let caret = element.selection_start().ok().flatten().unwrap_or(0);
    let initial = word_at(&value, caret);
    client(|client| client.rename_position = Some((path, line, character)));
    state.rename.set(Some(initial));
}

/// Sends the rename request for the stored position with the new name.
pub fn submit_rename(state: EditorState, new_name: &str) {
    state.rename.set(None);
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return;
    }
    let Some((path, line, character)) = client(|client| client.rename_position.clone()) else {
        return;
    };
    let id = next_id();
    client(|client| {
        client.pending.insert(id, Pending::Rename);
    });
    send_request_id(
        id,
        "textDocument/rename",
        json!({
            "textDocument": { "uri": file_uri(&path) },
            "position": { "line": line, "character": character },
            "newName": trimmed,
        }),
    );
}

/// Requests the code actions available at the caret into the action picker. The
/// diagnostics covering the caret line are passed as context, so the server
/// offers the quick-fixes tied to the error under the caret.
pub fn request_code_actions(state: EditorState) {
    let Some((path, line, character)) = caret_position(state) else {
        return;
    };
    let diagnostics = client(|client| {
        client
            .raw_diagnostics
            .get(&path)
            .map(|items| {
                items
                    .iter()
                    .filter(|diagnostic| covers_line(diagnostic, line))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });
    let id = next_id();
    client(|client| {
        client.pending.insert(id, Pending::CodeActions);
    });
    send_request_id(
        id,
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": file_uri(&path) },
            "range": {
                "start": { "line": line, "character": character },
                "end": { "line": line, "character": character },
            },
            "context": { "diagnostics": diagnostics },
        }),
    );
}

fn covers_line(diagnostic: &Value, line: u32) -> bool {
    let start = diagnostic
        .pointer("/range/start/line")
        .and_then(Value::as_u64);
    let end = diagnostic
        .pointer("/range/end/line")
        .and_then(Value::as_u64);
    matches!((start, end), (Some(start), Some(end)) if start <= line as u64 && line as u64 <= end)
}

/// Runs the code action chosen from the picker by index.
pub fn apply_code_action(state: EditorState, index: usize) {
    state.code_actions.set(Vec::new());
    let Some(action) = client(|client| client.code_actions.get(index).cloned()) else {
        return;
    };
    run_code_action(state, action);
}

/// Applies a code action: its edit if present, its command if present, and a
/// `codeAction/resolve` round trip if it carries neither but has resolve data.
fn run_code_action(state: EditorState, action: Value) {
    let has_edit = action.get("edit").is_some();
    if let Some(edit) = action.get("edit") {
        apply_workspace_edit(state, edit);
    }
    let command = if action.get("command").map(Value::is_object).unwrap_or(false) {
        action.get("command").cloned()
    } else if action.get("command").and_then(Value::as_str).is_some() {
        Some(action.clone())
    } else {
        None
    };
    if let Some(command) = &command {
        execute_command(command);
    }
    if !has_edit && command.is_none() && action.get("data").is_some() {
        let id = next_id();
        client(|client| {
            client.pending.insert(id, Pending::ResolveAction);
        });
        send_request_id(id, "codeAction/resolve", action);
    }
}

fn execute_command(command: &Value) {
    let Some(name) = command.get("command").and_then(Value::as_str) else {
        return;
    };
    let arguments = command.get("arguments").cloned().unwrap_or(json!([]));
    let id = next_id();
    client(|client| {
        client.pending.insert(id, Pending::Command);
    });
    send_request_id(
        id,
        "workspace/executeCommand",
        json!({ "command": name, "arguments": arguments }),
    );
}

/// Moves the caret to the next or previous diagnostic in the focused file.
pub fn goto_diagnostic(state: EditorState, forward: bool) {
    let buffer = state.focused_buffer();
    if buffer.kind != PluginKind::File {
        return;
    }
    let Some(path) = buffer.id else {
        return;
    };
    let diagnostics = client(|client| client.diagnostics.get(&path).cloned().unwrap_or_default());
    if diagnostics.is_empty() {
        return;
    }
    let Some(element) = crate::components::find::active() else {
        return;
    };
    let value = element.value();
    let caret = element.selection_start().ok().flatten().unwrap_or(0);
    let (line, _) = line_character(&value, caret);
    let current = line + 1;
    let mut lines: Vec<u32> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.line)
        .collect();
    lines.sort_unstable();
    lines.dedup();
    let target = if forward {
        lines
            .iter()
            .find(|candidate| **candidate > current)
            .copied()
            .or_else(|| lines.first().copied())
    } else {
        lines
            .iter()
            .rev()
            .find(|candidate| **candidate < current)
            .copied()
            .or_else(|| lines.last().copied())
    };
    if let Some(line) = target {
        state.goto.set(Some((path, line)));
    }
}

/// The focused Rust file and the caret's zero-based line and character.
fn caret_position(state: EditorState) -> Option<(String, u32, u32)> {
    if !ready() {
        return None;
    }
    let buffer = state.focused_buffer();
    if buffer.kind != PluginKind::File {
        return None;
    }
    let path = buffer.id?;
    if language_for_path(&path) != "rust" {
        return None;
    }
    let element = crate::components::find::active()?;
    let value = element.value();
    let caret = element.selection_start().ok().flatten().unwrap_or(0);
    let (line, character) = line_character(&value, caret);
    Some((path, line, character))
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
    let (line, column) = crate::caret::locate(&element, client_x, client_y);
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
    client(|client| client.suppress_completion = true);
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
    let Some(raw) = value.get("result").and_then(Value::as_array) else {
        return;
    };
    let edits = parse_edits(raw);
    apply_edits_to_file(state, path, &edits);
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

fn word_at(value: &str, caret: u32) -> String {
    let mut offset = 0;
    let mut current = String::new();
    let mut current_start = 0;
    for unit in value.chars() {
        let width = unit.len_utf16() as u32;
        if unit.is_alphanumeric() || unit == '_' {
            if current.is_empty() {
                current_start = offset;
            }
            current.push(unit);
        } else {
            if !current.is_empty() && caret >= current_start && caret <= offset {
                return current;
            }
            current.clear();
        }
        offset += width;
    }
    if !current.is_empty() && caret >= current_start && caret <= offset {
        return current;
    }
    String::new()
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
    let (x, top) = crate::caret::cell(element, line, column);
    (x, top + crate::caret::line_height(element))
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
