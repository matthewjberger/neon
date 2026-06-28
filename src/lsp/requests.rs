//! The outbound half of the LSP client: the request senders the editor
//! commands invoke (completion, hover, definitions, references, symbols,
//! rename, code actions, formatting), the code-action dispatch, diagnostic
//! navigation, and completion acceptance. Applying the server's replies and
//! the transport stay in `lsp.rs`.

use leptos::prelude::*;
use serde_json::{Value, json};

use super::edits::{apply_workspace_edit, offset_of, splice_utf16};
use super::text::{caret_pixel, line_character, word_at, word_prefix};
use super::transport::{file_uri, send_request_id};
use super::{Pending, client, next_id, ready, track};
use crate::state::{EditorState, PluginKind, language_for_path};

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
    let Some(element) = crate::components::overlays::find::active() else {
        return;
    };
    let value = element.value();
    let caret = element.selection_start().ok().flatten().unwrap_or(0);
    let (line, character) = line_character(&value, caret);
    let prefix = word_prefix(&value, caret);
    let (x, y) = caret_pixel(&element, line, character);
    let id = next_id();
    track(id, Pending::Completion { prefix, x, y });
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
    track(id, Pending::Definition);
    send_request_id(
        id,
        method,
        json!({
            "textDocument": { "uri": file_uri(&path) },
            "position": { "line": line, "character": character },
        }),
    );
}

/// Formats the focused Rust file and writes it once the edits land. Returns
/// false when there is no ready Rust file to format, so the caller can save
/// directly instead.
pub fn format_and_save(state: EditorState, path: &str) -> bool {
    if caret_position(state).is_none() {
        return false;
    }
    client(|client| {
        client.save_after_format.insert(path.to_string());
    });
    format_document(state);
    true
}

/// Requests a whole-document format from the server and applies the edits.
pub fn format_document(state: EditorState) {
    let Some((path, _, _)) = caret_position(state) else {
        return;
    };
    let id = next_id();
    track(id, Pending::Format { path: path.clone() });
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
    let Some(element) = crate::components::overlays::find::active() else {
        return;
    };
    let (x, y) = caret_pixel(&element, line, character);
    let id = next_id();
    track(id, Pending::Hover { x, y });
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
    let Some(element) = crate::components::overlays::find::active() else {
        return;
    };
    let (x, y) = caret_pixel(&element, line, character);
    let id = next_id();
    track(id, Pending::Signature { x, y });
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
    track(id, Pending::References);
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
    let query = crate::components::overlays::find::active()
        .map(|element| {
            let value = element.value();
            let caret = element.selection_start().ok().flatten().unwrap_or(0);
            word_at(&value, caret)
        })
        .unwrap_or_default();
    let id = next_id();
    track(
        id,
        Pending::Symbols {
            path: String::new(),
        },
    );
    send_request_id(id, "workspace/symbol", json!({ "query": query }));
}

/// Requests the document symbols of the focused file into the search panel.
pub fn request_symbols(state: EditorState) {
    let Some((path, _, _)) = caret_position(state) else {
        return;
    };
    let id = next_id();
    track(id, Pending::Symbols { path: path.clone() });
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
    let Some(element) = crate::components::overlays::find::active() else {
        return;
    };
    let value = element.value();
    let caret = element.selection_start().ok().flatten().unwrap_or(0);
    let initial = word_at(&value, caret);
    client(|client| client.rename_position = Some((path, line, character)));
    state.lsp.rename.set(Some(initial));
}

/// Sends the rename request for the stored position with the new name.
pub fn submit_rename(state: EditorState, new_name: &str) {
    state.lsp.rename.set(None);
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return;
    }
    let Some((path, line, character)) = client(|client| client.rename_position.clone()) else {
        return;
    };
    let id = next_id();
    track(id, Pending::Rename);
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
    track(id, Pending::CodeActions);
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

/// Requests the organize-imports source action for the focused file and applies
/// it directly, the comprehensive-LSP counterpart to formatting.
pub fn organize_imports(state: EditorState) {
    let Some((path, line, character)) = caret_position(state) else {
        return;
    };
    let id = next_id();
    track(id, Pending::OrganizeImports);
    send_request_id(
        id,
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": file_uri(&path) },
            "range": {
                "start": { "line": line, "character": character },
                "end": { "line": line, "character": character },
            },
            "context": {
                "diagnostics": [],
                "only": ["source.organizeImports"],
            },
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
    state.lsp.code_actions.set(Vec::new());
    let Some(action) = client(|client| client.code_actions.get(index).cloned()) else {
        return;
    };
    run_code_action(state, action);
}

/// Applies a code action: its edit if present, its command if present, and a
/// `codeAction/resolve` round trip if it carries neither but has resolve data.
pub(super) fn run_code_action(state: EditorState, action: Value) {
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
        track(id, Pending::ResolveAction);
        send_request_id(id, "codeAction/resolve", action);
    }
}

fn execute_command(command: &Value) {
    let Some(name) = command.get("command").and_then(Value::as_str) else {
        return;
    };
    let arguments = command.get("arguments").cloned().unwrap_or(json!([]));
    let id = next_id();
    track(id, Pending::Command);
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
    let Some(element) = crate::components::overlays::find::active() else {
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
        state.explorer.goto.set(Some((path, line)));
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
    let element = crate::components::overlays::find::active()?;
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
    let Some(element) = crate::components::overlays::find::active() else {
        return;
    };
    let (line, column) = crate::caret::locate(&element, client_x, client_y);
    let id = next_id();
    track(
        id,
        Pending::Hover {
            x: client_x,
            y: client_y,
        },
    );
    send_request_id(
        id,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": file_uri(&path) },
            "position": { "line": line, "character": column },
        }),
    );
}

/// Accepts a completion candidate, replacing the typed prefix and applying any
/// `additionalTextEdits` the server attached (an auto-import `use` line, say) in
/// the same change, with the caret tracked across edits that land above it.
pub fn accept_completion(state: EditorState, index: usize) {
    let Some(menu) = state.lsp.completion.get_untracked() else {
        return;
    };
    let Some(entry) = menu.items.get(index) else {
        return;
    };
    let Some(element) = crate::components::overlays::find::active() else {
        return;
    };
    let value = element.value();
    let caret = element.selection_start().ok().flatten().unwrap_or(0);
    let prefix_units = menu.prefix.encode_utf16().count() as u32;
    let start = caret.saturating_sub(prefix_units);

    // The prefix replacement plus the server's extra edits, all as UTF-16 ranges
    // over the same text so they splice together cleanly.
    let mut edits: Vec<(u32, u32, String)> = vec![(start, caret, entry.insert.clone())];
    for raw in &entry.additional_edits {
        if let Some(resolved) = resolve_text_edit(&value, raw) {
            edits.push(resolved);
        }
    }

    // The caret lands after the inserted text, pushed right by every extra edit
    // that resolves before the insertion point (the import lines above it).
    let insert_units = entry.insert.encode_utf16().count() as i64;
    let shift: i64 = edits
        .iter()
        .skip(1)
        .filter(|(_, end, _)| *end <= start)
        .map(|(edit_start, edit_end, text)| {
            text.encode_utf16().count() as i64 - (*edit_end as i64 - *edit_start as i64)
        })
        .sum();
    let new_caret = (start as i64 + insert_units + shift).max(0) as u32;

    // Splice from the end so earlier offsets stay valid as later ones change.
    edits.sort_by_key(|(edit_start, _, _)| std::cmp::Reverse(*edit_start));
    let mut replaced = value;
    for (edit_start, edit_end, text) in &edits {
        replaced = splice_utf16(&replaced, *edit_start, *edit_end, text);
    }

    element.set_value(&replaced);
    let _ = element.set_selection_range(new_caret, new_caret);
    let _ = element.focus();
    client(|client| client.suppress_completion = true);
    if let Ok(event) = web_sys::Event::new("input") {
        let _ = element.dispatch_event(&event);
    }
    state.lsp.completion.set(None);
}

/// Resolves one LSP text edit to a UTF-16 `(start, end, new_text)` range over
/// `value`, the form [`accept_completion`] splices.
fn resolve_text_edit(value: &str, edit: &Value) -> Option<(u32, u32, String)> {
    let start = offset_of(
        value,
        edit.pointer("/range/start/line").and_then(Value::as_u64)? as u32,
        edit.pointer("/range/start/character")
            .and_then(Value::as_u64)? as u32,
    );
    let end = offset_of(
        value,
        edit.pointer("/range/end/line").and_then(Value::as_u64)? as u32,
        edit.pointer("/range/end/character")
            .and_then(Value::as_u64)? as u32,
    );
    let new_text = edit.get("newText").and_then(Value::as_str)?.to_string();
    Some((start, end, new_text))
}
