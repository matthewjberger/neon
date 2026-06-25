//! Multi-cursor editing layered on the textarea. Extra carets are UTF-16 offsets
//! in the buffer; an edit applies at the primary caret and every extra at once.
//! This is column editing: add a caret above or below, then type, backspace, or
//! delete at all of them. Offsets are kept and recomputed in UTF-16 units, the
//! units the textarea selection uses.

use leptos::prelude::*;
use web_sys::HtmlTextAreaElement;

use crate::components::overlays::find;
use crate::state::EditorState;

/// Whether multi-cursor mode is on (any extra carets).
pub fn active(state: EditorState) -> bool {
    state
        .editing
        .cursors
        .with_untracked(|cursors| !cursors.is_empty())
}

/// Clears the extra carets.
pub fn clear(state: EditorState) {
    if active(state) {
        state.editing.cursors.set(Vec::new());
    }
}

/// Adds a caret one line below the primary caret, at the same column.
pub fn add_below(state: EditorState) {
    add_line(state, 1);
}

/// Adds a caret one line above the primary caret, at the same column.
pub fn add_above(state: EditorState) {
    add_line(state, -1);
}

fn add_line(state: EditorState, delta: i64) {
    let Some(element) = find::active() else {
        return;
    };
    let value = element.value();
    let base = primary(&element);
    let (line, column) = line_col(&value, base);
    let target = line as i64 + delta;
    if target < 0 {
        return;
    }
    if let Some(offset) = offset_at(&value, target as u32, column)
        && offset != base
    {
        state.editing.cursors.update(|cursors| {
            if !cursors.contains(&offset) {
                cursors.push(offset);
            }
        });
    }
}

/// Inserts text at the primary caret and every extra caret.
pub fn insert(state: EditorState, text: &str) {
    let Some(element) = find::active() else {
        return;
    };
    let base = primary(&element);
    let positions = positions(state, base);
    let inserted: Vec<u16> = text.encode_utf16().collect();
    let length = inserted.len() as u32;
    let mut units: Vec<u16> = element.value().encode_utf16().collect();
    for &position in positions.iter().rev() {
        let at = (position as usize).min(units.len());
        units.splice(at..at, inserted.iter().copied());
    }
    let moved: Vec<u32> = positions
        .iter()
        .enumerate()
        .map(|(index, position)| position + length * (index as u32 + 1))
        .collect();
    apply(state, &element, &units, base, &positions, &moved);
}

/// Deletes the character before the primary caret and every extra caret.
pub fn delete_back(state: EditorState) {
    let Some(element) = find::active() else {
        return;
    };
    let base = primary(&element);
    let positions = positions(state, base);
    let mut units: Vec<u16> = element.value().encode_utf16().collect();
    let removals: Vec<u32> = positions
        .iter()
        .filter(|position| **position > 0)
        .map(|position| position - 1)
        .collect();
    for &index in removals.iter().rev() {
        if (index as usize) < units.len() {
            units.remove(index as usize);
        }
    }
    let mut removed = 0;
    let moved: Vec<u32> = positions
        .iter()
        .map(|position| {
            if *position > 0 {
                removed += 1;
            }
            position.saturating_sub(removed)
        })
        .collect();
    apply(state, &element, &units, base, &positions, &moved);
}

/// Deletes the character after the primary caret and every extra caret.
pub fn delete_forward(state: EditorState) {
    let Some(element) = find::active() else {
        return;
    };
    let base = primary(&element);
    let positions = positions(state, base);
    let mut units: Vec<u16> = element.value().encode_utf16().collect();
    let length = units.len() as u32;
    for &position in positions.iter().rev() {
        if position < length {
            units.remove(position as usize);
        }
    }
    let mut before = 0;
    let moved: Vec<u32> = positions
        .iter()
        .map(|position| {
            let shifted = position.saturating_sub(before);
            if *position < length {
                before += 1;
            }
            shifted
        })
        .collect();
    apply(state, &element, &units, base, &positions, &moved);
}

/// The sorted, unique set of edit positions: the primary caret plus the extras.
fn positions(state: EditorState, base: u32) -> Vec<u32> {
    let mut positions = state.editing.cursors.get_untracked();
    positions.push(base);
    positions.sort_unstable();
    positions.dedup();
    positions
}

/// Writes the new text back, dispatches input so the commit path runs, and sets
/// the primary selection and the extra carets from the moved positions.
fn apply(
    state: EditorState,
    element: &HtmlTextAreaElement,
    units: &[u16],
    base: u32,
    positions: &[u32],
    moved: &[u32],
) {
    let value = String::from_utf16_lossy(units);
    element.set_value(&value);
    if let Ok(event) = web_sys::Event::new("input") {
        let _ = element.dispatch_event(&event);
    }
    let index = positions.iter().position(|p| *p == base).unwrap_or(0);
    let caret = moved.get(index).copied().unwrap_or(0);
    let _ = element.set_selection_range(caret, caret);
    let _ = element.focus();
    let extras: Vec<u32> = moved
        .iter()
        .enumerate()
        .filter(|(position_index, _)| *position_index != index)
        .map(|(_, offset)| *offset)
        .collect();
    state.editing.cursors.set(extras);
}

fn primary(element: &HtmlTextAreaElement) -> u32 {
    element.selection_start().ok().flatten().unwrap_or(0)
}

/// The line and UTF-16 column of an offset.
pub fn line_col(value: &str, offset: u32) -> (u32, u32) {
    let mut line = 0;
    let mut column = 0;
    let mut seen = 0;
    for character in value.chars() {
        if seen >= offset {
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

/// The UTF-16 offset at a line and column, clamped to the line length, or None
/// when the line does not exist.
fn offset_at(value: &str, line: u32, column: u32) -> Option<u32> {
    const NEWLINE: u16 = b'\n' as u16;
    let units: Vec<u16> = value.encode_utf16().collect();
    let mut current_line = 0;
    let mut start = if line == 0 { Some(0) } else { None };
    for (index, unit) in units.iter().enumerate() {
        if *unit == NEWLINE {
            current_line += 1;
            if current_line == line {
                start = Some(index + 1);
            }
        }
    }
    let start = start?;
    let end = units[start..]
        .iter()
        .position(|unit| *unit == NEWLINE)
        .map(|offset| start + offset)
        .unwrap_or(units.len());
    let line_length = (end - start) as u32;
    Some(start as u32 + column.min(line_length))
}
