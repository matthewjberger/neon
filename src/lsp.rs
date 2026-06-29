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
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use web_sys::WebSocket;

use crate::state::{
    CompletionEntry, CompletionMenu, EditorState, HierarchyEntry, HoverCard, InlayHint,
    OutlineNode, PluginKind, SidebarView, basename, language_for_path, lsp_language_for,
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
    organize_imports, request_call_hierarchy, request_code_actions, request_code_lenses,
    request_completion, request_folding_ranges, request_hover_at, request_hover_at_caret,
    request_inlay_hints, request_locations, request_outline, request_references,
    request_signature_help, request_symbols, request_type_hierarchy, request_workspace_symbols,
    start_rename, submit_rename,
};
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
    Outline { path: String },
    CallHierarchyPrepare { incoming: bool },
    CallHierarchyCalls { incoming: bool },
    TypeHierarchyPrepare { supertypes: bool },
    TypeHierarchyCalls { supertypes: bool },
    InlayHints { path: String },
    FoldingRanges { path: String },
    CodeLenses { path: String },
    Rename,
    CodeActions,
    OrganizeImports,
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
    /// The last text synced to the server per file, so `didChange` can send the
    /// minimal changed range instead of the whole document every keystroke.
    last_text: HashMap<String, String>,
    /// The pending debounce timer for the per-edit feature requests (inlay
    /// hints, folding, code lenses), so they fire once the user pauses.
    feature_timer: Option<i32>,
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
            last_text: HashMap::new(),
            feature_timer: None,
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

/// The full set of symbol and completion-item kinds, so the server is free to
/// answer with any of them rather than just the LSP 1.0 subset.
const SYMBOL_KINDS: [u64; 26] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
];
const COMPLETION_KINDS: [u64; 25] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
];

/// The `initialize` request, advertising the full client surface the editor
/// actually consumes: completion with snippets and resolve, hover and signature
/// help in markdown, the four goto kinds with link support, hierarchical symbols,
/// code actions with literal and resolve support, prepare-aware rename, rich
/// diagnostics, and work-done progress. Advertising these is what makes
/// rust-analyzer offer auto-imports, quick-fixes, and indexing progress at all.
fn initialize_params(state: EditorState) -> Value {
    json!({
        "processId": Value::Null,
        "rootUri": state.explorer.root.get_untracked().map(|root| file_uri(&root)),
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": {
                "applyEdit": true,
                "workspaceEdit": {
                    "documentChanges": true,
                    "resourceOperations": ["create", "rename", "delete"],
                    "failureHandling": "abort",
                },
                "configuration": true,
                "didChangeConfiguration": { "dynamicRegistration": true },
                "didChangeWatchedFiles": { "dynamicRegistration": true },
                "executeCommand": { "dynamicRegistration": true },
                "workspaceFolders": true,
                "symbol": { "symbolKind": { "valueSet": SYMBOL_KINDS } },
            },
            "textDocument": {
                "synchronization": {
                    "didSave": true,
                    "willSave": false,
                    "willSaveWaitUntil": false,
                    "dynamicRegistration": false,
                },
                "publishDiagnostics": {
                    "relatedInformation": true,
                    "tagSupport": { "valueSet": [1, 2] },
                    "codeDescriptionSupport": true,
                    "dataSupport": true,
                },
                "completion": {
                    "dynamicRegistration": false,
                    "contextSupport": true,
                    "completionItem": {
                        "snippetSupport": false,
                        "documentationFormat": ["markdown", "plaintext"],
                        "deprecatedSupport": true,
                        "preselectSupport": true,
                        "labelDetailsSupport": true,
                        "resolveSupport": { "properties": ["documentation", "detail"] },
                    },
                    "completionItemKind": { "valueSet": COMPLETION_KINDS },
                },
                "hover": { "contentFormat": ["markdown", "plaintext"] },
                "signatureHelp": {
                    "signatureInformation": {
                        "documentationFormat": ["markdown", "plaintext"],
                        "parameterInformation": { "labelOffsetSupport": true },
                        "activeParameterSupport": true,
                    },
                },
                "definition": { "linkSupport": true },
                "typeDefinition": { "linkSupport": true },
                "implementation": { "linkSupport": true },
                "declaration": { "linkSupport": true },
                "references": {},
                "documentHighlight": {},
                "documentSymbol": {
                    "hierarchicalDocumentSymbolSupport": true,
                    "symbolKind": { "valueSet": SYMBOL_KINDS },
                },
                "codeAction": {
                    "dynamicRegistration": false,
                    "isPreferredSupport": true,
                    "dataSupport": true,
                    "resolveSupport": { "properties": ["edit"] },
                    "codeActionLiteralSupport": {
                        "codeActionKind": {
                            "valueSet": [
                                "", "quickfix", "refactor", "refactor.extract",
                                "refactor.inline", "refactor.rewrite", "source",
                                "source.organizeImports", "source.fixAll",
                            ],
                        },
                    },
                },
                "rename": { "prepareSupport": true, "dynamicRegistration": false },
                "formatting": {},
                "rangeFormatting": {},
                "foldingRange": { "lineFoldingOnly": true },
                "selectionRange": {},
                "callHierarchy": {},
                "typeHierarchy": {},
                "inlayHint": { "resolveSupport": { "properties": [] } },
                "codeLens": { "dynamicRegistration": false },
            },
            "window": {
                "workDoneProgress": true,
                "showMessage": {},
                "showDocument": { "support": false },
            },
        },
    })
}

/// Shows the consent toast for the first LSP-capable file, unless a server is
/// already running. The first such file picks the workspace's server family;
/// files of other families are left to the language worker.
pub fn did_open(state: EditorState, path: &str) {
    let Some(family) = lsp_language_for(language_for_path(path)) else {
        return;
    };
    match state.lsp.language.get_untracked() {
        Some(active) if active != family => {}
        Some(_) => {
            if state.lsp.started.get_untracked() {
                open_document(state, path);
            } else {
                state.lsp.consent.set(true);
            }
        }
        None => {
            state.lsp.language.set(Some(family.to_string()));
            state.lsp.consent.set(true);
        }
    }
}

/// Accepts the consent toast: enables the bridge and starts the handshake.
pub fn enable(state: EditorState) {
    state.lsp.consent.set(false);
    if state.lsp.started.get_untracked() {
        return;
    }
    state.lsp.started.set(true);
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
    let (change, version) = client(|client| {
        let last = client.last_text.get(path).cloned().unwrap_or_default();
        let change = incremental_change(&last, &text);
        client.last_text.insert(path.to_string(), text);
        let entry = client.versions.entry(path.to_string()).or_insert(0);
        *entry += 1;
        (change, *entry)
    });
    let Some(change) = change else {
        return;
    };
    notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": file_uri(path), "version": version },
            "contentChanges": [change],
        }),
    );
    schedule_feature_refresh(state, path);
}

/// The minimal LSP `contentChanges` entry turning `old` into `new`: the changed
/// range (in old-text UTF-16 positions) and its replacement, found by trimming
/// the common prefix and suffix. `None` when the texts are identical.
fn incremental_change(old: &str, new: &str) -> Option<Value> {
    let old_chars: Vec<char> = old.chars().collect();
    let new_chars: Vec<char> = new.chars().collect();
    let bound = old_chars.len().min(new_chars.len());
    let mut prefix = 0;
    while prefix < bound && old_chars[prefix] == new_chars[prefix] {
        prefix += 1;
    }
    let tail = (old_chars.len() - prefix).min(new_chars.len() - prefix);
    let mut suffix = 0;
    while suffix < tail
        && old_chars[old_chars.len() - 1 - suffix] == new_chars[new_chars.len() - 1 - suffix]
    {
        suffix += 1;
    }
    if prefix == old_chars.len() && prefix == new_chars.len() {
        return None;
    }
    let old_end = old_chars.len() - suffix;
    let new_end = new_chars.len() - suffix;
    let replacement: String = new_chars[prefix..new_end].iter().collect();
    let start = position_in(&old_chars, prefix);
    let end = position_in(&old_chars, old_end);
    Some(json!({
        "range": {
            "start": { "line": start.0, "character": start.1 },
            "end": { "line": end.0, "character": end.1 },
        },
        "text": replacement,
    }))
}

/// The `(line, utf16-character)` position of a character offset in a char slice,
/// for the incremental change range.
fn position_in(chars: &[char], offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut character = 0u32;
    for &value in &chars[..offset.min(chars.len())] {
        if value == '\n' {
            line += 1;
            character = 0;
        } else {
            character += value.len_utf16() as u32;
        }
    }
    (line, character)
}

/// Schedules the per-edit feature requests (inlay hints, folding, code lenses)
/// to fire once typing pauses, replacing any pending timer, so a burst of
/// keystrokes makes one request each instead of one per character.
fn schedule_feature_refresh(state: EditorState, path: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    client(|client| {
        if let Some(handle) = client.feature_timer.take() {
            window.clear_timeout_with_handle(handle);
        }
    });
    let path = path.to_string();
    let callback = Closure::once_into_js(move || {
        client(|client| client.feature_timer = None);
        request_inlay_hints(state, &path);
        request_folding_ranges(&path);
        request_code_lenses(&path);
    });
    if let Ok(handle) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        250,
    ) {
        client(|client| client.feature_timer = Some(handle));
    }
}

/// Notifies the server a file was saved, carrying its text so server-side
/// save hooks (rust-analyzer's check-on-save) run against the saved content.
pub fn did_save(state: EditorState, path: &str) {
    if !ready() {
        return;
    }
    let open = client(|client| client.versions.contains_key(path));
    if !open {
        return;
    }
    let text = state.buffer_source(PluginKind::File, &Some(path.to_string()));
    notify(
        "textDocument/didSave",
        json!({
            "textDocument": { "uri": file_uri(path) },
            "text": text,
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
        state.lsp.completion.set(None);
        return;
    }
    state.lsp.completion.set(Some(CompletionMenu {
        items,
        x,
        y,
        prefix,
    }));
    state.lsp.completion_index.set(0);
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
    let additional_edits = item
        .get("additionalTextEdits")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Some(CompletionEntry {
        label,
        insert,
        detail,
        kind,
        additional_edits,
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
            state.explorer.goto.set(Some((path.clone(), *line)));
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
            state.lsp.symbol_picker.set(hits);
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
    state.explorer.search_results.set(hits);
    state.sidebar_view.set(SidebarView::Search);
}

fn apply_symbols(state: EditorState, value: &Value, path: &str) {
    let Some(items) = value.get("result").and_then(Value::as_array) else {
        return;
    };
    let mut hits = Vec::new();
    collect_symbols(items, path, &mut hits);
    state.lsp.symbol_picker.set(hits);
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

fn apply_outline(state: EditorState, value: &Value, path: &str) {
    let nodes = value
        .get("result")
        .and_then(Value::as_array)
        .map(|items| build_outline(items))
        .unwrap_or_default();
    state.lsp.outline_path.set(path.to_string());
    state.lsp.outline.set(nodes);
}

fn build_outline(items: &[Value]) -> Vec<OutlineNode> {
    items
        .iter()
        .filter_map(|item| {
            let name = item.get("name").and_then(Value::as_str)?.to_string();
            let kind = item.get("kind").and_then(Value::as_u64).unwrap_or(0) as u8;
            let line = item
                .pointer("/selectionRange/start/line")
                .or_else(|| item.pointer("/range/start/line"))
                .or_else(|| item.pointer("/location/range/start/line"))
                .and_then(Value::as_u64)? as u32;
            let children = item
                .get("children")
                .and_then(Value::as_array)
                .map(|nested| build_outline(nested))
                .unwrap_or_default();
            Some(OutlineNode {
                name,
                kind,
                line,
                children,
            })
        })
        .collect()
}

/// Parses an `inlayHint` reply into the per-file hint list the surface draws.
/// A hint's label is either a string or an array of parts with `value` fields.
fn apply_inlay_hints(state: EditorState, value: &Value, path: &str) {
    let Some(items) = value.get("result").and_then(Value::as_array) else {
        return;
    };
    let hints = items
        .iter()
        .filter_map(|item| {
            let line = item.pointer("/position/line").and_then(Value::as_u64)? as u32;
            let character = item
                .pointer("/position/character")
                .and_then(Value::as_u64)? as u32;
            let label = inlay_label(item.get("label")?);
            if label.is_empty() {
                return None;
            }
            Some(InlayHint {
                line,
                character,
                label,
                padding_left: item
                    .get("paddingLeft")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                padding_right: item
                    .get("paddingRight")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect::<Vec<_>>();
    state.lsp.inlay_hints.update(|map| {
        map.insert(path.to_string(), hints);
    });
}

/// Parses a `foldingRange` reply into the per-file `(start, end)` line ranges the
/// surface gutter offers as fold toggles.
fn apply_folding_ranges(state: EditorState, value: &Value, path: &str) {
    let Some(items) = value.get("result").and_then(Value::as_array) else {
        return;
    };
    let ranges = items
        .iter()
        .filter_map(|item| {
            let start = item.get("startLine").and_then(Value::as_u64)? as u32;
            let end = item.get("endLine").and_then(Value::as_u64)? as u32;
            (end > start).then_some((start, end))
        })
        .collect::<Vec<_>>();
    state.lsp.folding_ranges.update(|map| {
        map.insert(path.to_string(), ranges);
    });
}

/// Parses a `codeLens` reply into the `(line, title)` labels the surface draws
/// above lines. Only lenses that arrive with a resolved command title are kept;
/// lenses needing a separate resolve round-trip are skipped.
fn apply_code_lenses(state: EditorState, value: &Value, path: &str) {
    let Some(items) = value.get("result").and_then(Value::as_array) else {
        return;
    };
    let lenses = items
        .iter()
        .filter_map(|item| {
            let line = item.pointer("/range/start/line").and_then(Value::as_u64)? as u32;
            let title = item
                .pointer("/command/title")
                .and_then(Value::as_str)
                .filter(|title| !title.is_empty())?
                .to_string();
            Some((line, title))
        })
        .collect::<Vec<_>>();
    state.lsp.code_lenses.update(|map| {
        map.insert(path.to_string(), lenses);
    });
}

/// The display text of an inlay-hint label: a bare string, or the joined `value`
/// fields of its parts.
fn inlay_label(label: &Value) -> String {
    if let Some(text) = label.as_str() {
        return text.to_string();
    }
    label
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("value").and_then(Value::as_str))
                .collect::<String>()
        })
        .unwrap_or_default()
}

/// The first reply of the two-step call-hierarchy gesture. `prepareCallHierarchy`
/// resolves the symbol under the caret to a `CallHierarchyItem`; we hand that
/// item straight back as `callHierarchy/incomingCalls` (or `outgoingCalls`) to
/// list its callers or callees.
fn apply_call_hierarchy_prepare(value: &Value, incoming: bool) {
    let Some(item) = value
        .get("result")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .cloned()
    else {
        return;
    };
    let method = if incoming {
        "callHierarchy/incomingCalls"
    } else {
        "callHierarchy/outgoingCalls"
    };
    let id = next_id();
    track(id, Pending::CallHierarchyCalls { incoming });
    send_request_id(id, method, json!({ "item": item }));
}

/// The second reply: the incoming or outgoing calls. Each entry wraps the related
/// `CallHierarchyItem` under `from` (callers) or `to` (callees); flatten those to
/// rows that jump to the related symbol.
fn apply_call_hierarchy_calls(state: EditorState, value: &Value, incoming: bool) {
    let key = if incoming { "from" } else { "to" };
    let entries = value
        .get("result")
        .and_then(Value::as_array)
        .map(|calls| {
            calls
                .iter()
                .filter_map(|call| hierarchy_entry(call.get(key)?))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    state.lsp.call_hierarchy_incoming.set(incoming);
    state.lsp.call_hierarchy.set(entries);
}

/// The first reply of the type-hierarchy gesture. `prepareTypeHierarchy` resolves
/// the symbol under the caret; we hand the item back as `typeHierarchy/supertypes`
/// or `/subtypes` to list its parents or children in the type graph.
fn apply_type_hierarchy_prepare(value: &Value, supertypes: bool) {
    let Some(item) = value
        .get("result")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .cloned()
    else {
        return;
    };
    let method = if supertypes {
        "typeHierarchy/supertypes"
    } else {
        "typeHierarchy/subtypes"
    };
    let id = next_id();
    track(id, Pending::TypeHierarchyCalls { supertypes });
    send_request_id(id, method, json!({ "item": item }));
}

/// The second reply: the supertypes or subtypes, each a `TypeHierarchyItem`
/// directly (not wrapped), flattened to rows that jump to the related type.
fn apply_type_hierarchy_calls(state: EditorState, value: &Value, supertypes: bool) {
    let entries = value
        .get("result")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(hierarchy_entry).collect::<Vec<_>>())
        .unwrap_or_default();
    state.lsp.type_hierarchy_super.set(supertypes);
    state.lsp.type_hierarchy.set(entries);
}

fn hierarchy_entry(item: &Value) -> Option<HierarchyEntry> {
    let name = item.get("name").and_then(Value::as_str)?.to_string();
    let detail = item
        .get("detail")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let path = item.get("uri").and_then(Value::as_str).map(path_from_uri)?;
    let line = item
        .pointer("/selectionRange/start/line")
        .or_else(|| item.pointer("/range/start/line"))
        .and_then(Value::as_u64)? as u32
        + 1;
    Some(HierarchyEntry {
        name,
        detail,
        path,
        line,
    })
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
    state.lsp.code_actions.set(titles);
}

/// Applies the first organize-imports action the server returns, sorting and
/// merging the file's `use` declarations without opening the action picker.
fn apply_organize_imports(state: EditorState, value: &Value) {
    let Some(action) = value
        .get("result")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
    else {
        return;
    };
    run_code_action(state, action.clone());
}

fn apply_signature(state: EditorState, value: &Value, x: f64, y: f64) {
    let result = value.get("result");
    let signatures = result
        .and_then(|result| result.get("signatures"))
        .and_then(Value::as_array);
    let Some(signatures) = signatures.filter(|items| !items.is_empty()) else {
        state.lsp.hover.set(None);
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
        state.lsp.hover.set(None);
    } else {
        state.lsp.hover.set(Some(HoverCard {
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
        state.lsp.hover.set(None);
    } else {
        state.lsp.hover.set(Some(HoverCard { text, x, y }));
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
            send_request("initialize", initialize_params(state));
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
        // rust-analyzer pulls its settings; answer with nulls so it uses its
        // defaults instead of blocking on a reply that never comes.
        Some("workspace/configuration") => {
            let count = value
                .pointer("/params/items")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            let result = vec![Value::Null; count];
            if let Some(id) = value.get("id") {
                send_raw(json!({ "jsonrpc": "2.0", "id": id, "result": result }));
            }
            return;
        }
        // Dynamic (un)registration and progress tokens: acknowledge so the server
        // proceeds. We register nothing of our own to act on.
        Some("client/registerCapability")
        | Some("client/unregisterCapability")
        | Some("window/workDoneProgress/create")
        | Some("window/showMessageRequest") => {
            if let Some(id) = value.get("id") {
                send_raw(json!({ "jsonrpc": "2.0", "id": id, "result": Value::Null }));
            }
            return;
        }
        // Indexing and build progress, and the server's own messages, into the
        // LSP log so the status is visible while rust-analyzer warms up.
        Some("$/progress") => {
            log_progress(state, &value);
            return;
        }
        Some("window/showMessage") | Some("window/logMessage") => {
            if let Some(message) = value.pointer("/params/message").and_then(Value::as_str) {
                log(state, message.to_string());
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
            Pending::Outline { path } => apply_outline(state, &value, &path),
            Pending::CallHierarchyPrepare { incoming } => {
                apply_call_hierarchy_prepare(&value, incoming)
            }
            Pending::CallHierarchyCalls { incoming } => {
                apply_call_hierarchy_calls(state, &value, incoming)
            }
            Pending::TypeHierarchyPrepare { supertypes } => {
                apply_type_hierarchy_prepare(&value, supertypes)
            }
            Pending::TypeHierarchyCalls { supertypes } => {
                apply_type_hierarchy_calls(state, &value, supertypes)
            }
            Pending::InlayHints { path } => apply_inlay_hints(state, &value, &path),
            Pending::FoldingRanges { path } => apply_folding_ranges(state, &value, &path),
            Pending::CodeLenses { path } => apply_code_lenses(state, &value, &path),
            Pending::Rename => {
                if let Some(result) = value.get("result") {
                    apply_workspace_edit(state, result);
                }
            }
            Pending::CodeActions => apply_code_actions(state, &value),
            Pending::OrganizeImports => apply_organize_imports(state, &value),
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
        notify(
            "workspace/didChangeConfiguration",
            json!({ "settings": {} }),
        );
        client(|client| client.ready = true);
        for path in open_rust_files(state) {
            open_document(state, &path);
        }
    }
}

/// Turns a `$/progress` notification into one log line: the work's title and the
/// percentage or message it reports, so a long index reads as progress.
fn log_progress(state: EditorState, value: &Value) {
    let payload = value.pointer("/params/value");
    let Some(payload) = payload else {
        return;
    };
    let kind = payload.get("kind").and_then(Value::as_str).unwrap_or("");
    let title = payload.get("title").and_then(Value::as_str);
    let message = payload.get("message").and_then(Value::as_str);
    let percentage = payload.get("percentage").and_then(Value::as_u64);
    let detail = match (message, percentage) {
        (Some(message), Some(percentage)) => format!("{message} ({percentage}%)"),
        (Some(message), None) => message.to_string(),
        (None, Some(percentage)) => format!("{percentage}%"),
        (None, None) => String::new(),
    };
    let label = title.unwrap_or("progress");
    match kind {
        "end" => log(state, format!("{label}: done")),
        _ if detail.is_empty() => {}
        _ => log(state, format!("{label}: {detail}")),
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
                "languageId": language_for_path(path),
                "version": 0,
                "text": text.clone(),
            }
        }),
    );
    client(|client| {
        client.last_text.insert(path.to_string(), text);
    });
    request_inlay_hints(state, path);
    request_folding_ranges(path);
    request_code_lenses(path);
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
    state.lsp.problems.set(problems);
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
    state.lsp.log.update(|entries| {
        entries.push(line);
        let overflow = entries.len().saturating_sub(500);
        if overflow > 0 {
            entries.drain(0..overflow);
        }
    });
}
