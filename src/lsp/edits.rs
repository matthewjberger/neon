//! The workspace-edit engine. Parse the server's ranged edits, resolve each
//! range to a unit offset, and splice them into a buffer from the end so earlier
//! offsets stay valid. Edits for a file that is not open are queued until it
//! loads, then applied.

use leptos::prelude::*;
use serde_json::Value;

use super::transport::path_from_uri;
use super::{client, did_change};
use crate::state::{EditorState, PluginKind};

/// One ranged text edit from the server, before it is resolved to unit offsets
/// against a specific document version.
pub(super) struct RangeEdit {
    start_line: u32,
    start_character: u32,
    end_line: u32,
    end_character: u32,
    new_text: String,
}

pub(super) fn parse_edits(raw: &[Value]) -> Vec<RangeEdit> {
    raw.iter().filter_map(to_range_edit).collect()
}

fn to_range_edit(edit: &Value) -> Option<RangeEdit> {
    Some(RangeEdit {
        start_line: edit.pointer("/range/start/line").and_then(Value::as_u64)? as u32,
        start_character: edit
            .pointer("/range/start/character")
            .and_then(Value::as_u64)? as u32,
        end_line: edit.pointer("/range/end/line").and_then(Value::as_u64)? as u32,
        end_character: edit
            .pointer("/range/end/character")
            .and_then(Value::as_u64)? as u32,
        new_text: edit.get("newText").and_then(Value::as_str)?.to_string(),
    })
}

/// Applies ranged edits to a string, resolving each range to a unit offset and
/// splicing from the end so earlier offsets stay valid.
fn apply_range_edits(text: &str, edits: &[RangeEdit]) -> String {
    let mut resolved: Vec<(u32, u32, &str)> = edits
        .iter()
        .map(|edit| {
            (
                offset_of(text, edit.start_line, edit.start_character),
                offset_of(text, edit.end_line, edit.end_character),
                edit.new_text.as_str(),
            )
        })
        .collect();
    resolved.sort_by_key(|edit| std::cmp::Reverse(edit.0));
    let mut result = text.to_string();
    for (start, end, new_text) in resolved {
        result = splice_utf16(&result, start, end, new_text);
    }
    result
}

/// Applies edits to an open file's buffer and notifies the server.
pub(super) fn apply_edits_to_file(state: EditorState, path: &str, edits: &[RangeEdit]) {
    if edits.is_empty() {
        return;
    }
    let text = state.buffer_source(PluginKind::File, &Some(path.to_string()));
    let result = apply_range_edits(&text, edits);
    state.set_buffer_text(PluginKind::File, &Some(path.to_string()), result);
    did_change(state, path);
}

fn offset_of(value: &str, line: u32, character: u32) -> u32 {
    let mut current_line = 0;
    let mut offset = 0;
    for unit in value.chars() {
        if current_line == line {
            break;
        }
        if unit == '\n' {
            current_line += 1;
        }
        offset += unit.len_utf16() as u32;
    }
    offset + character
}

pub(super) fn splice_utf16(value: &str, start: u32, end: u32, replacement: &str) -> String {
    let units: Vec<u16> = value.encode_utf16().collect();
    let head = String::from_utf16_lossy(&units[..start as usize]);
    let tail = String::from_utf16_lossy(&units[end as usize..]);
    format!("{head}{replacement}{tail}")
}

pub(super) fn apply_workspace_edit(state: EditorState, edit: &Value) {
    if let Some(changes) = edit.get("changes").and_then(Value::as_object) {
        for (uri, edits) in changes {
            if let Some(array) = edits.as_array() {
                apply_uri_edits(state, uri, array);
            }
        }
    }
    if let Some(document_changes) = edit.get("documentChanges").and_then(Value::as_array) {
        for change in document_changes {
            let uri = change.pointer("/textDocument/uri").and_then(Value::as_str);
            let edits = change.get("edits").and_then(Value::as_array);
            if let (Some(uri), Some(edits)) = (uri, edits) {
                apply_uri_edits(state, uri, edits);
            }
        }
    }
}

fn apply_uri_edits(state: EditorState, uri: &str, raw: &[Value]) {
    let path = path_from_uri(uri);
    let edits = parse_edits(raw);
    if edits.is_empty() {
        return;
    }
    let open = state
        .files
        .with_untracked(|files| files.iter().any(|file| file.path == path));
    if open {
        apply_edits_to_file(state, &path, &edits);
    } else {
        client(|client| {
            client.pending_edits.insert(path.clone(), edits);
        });
        crate::fs::read_file(&path);
    }
}

/// Applies edits queued for a file that was not open when a workspace edit
/// arrived, called once the file's content is loaded.
pub fn apply_pending_edits(state: EditorState, path: &str) {
    let edits = client(|client| client.pending_edits.remove(path));
    if let Some(edits) = edits {
        apply_edits_to_file(state, path, &edits);
    }
}
