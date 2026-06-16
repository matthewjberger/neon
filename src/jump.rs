//! Avy-style jump: label every word start or line start on screen, show the
//! labels over the buffer, and jump the caret to the one whose label you type.
//! Spacemacs' `SPC j w` and `SPC j l`. The labels are the sentinels: type a
//! label to jump there, Escape to cancel.

use leptos::prelude::*;

use crate::caret;
use crate::components::find;
use crate::state::{EditorState, JumpState, JumpTarget};

const LABELS: &[u8] = b"asdfghjklqwertyuiopzxcvbnm";
const LIMIT: usize = 120;

/// What to jump to.
pub enum JumpKind {
    Word,
    Line,
}

/// A candidate position: its UTF-16 caret offset and document line and column.
struct Spot {
    offset: u32,
    line: u32,
    column: u32,
}

/// Enters jump mode for the focused buffer.
pub fn start(state: EditorState, kind: JumpKind) {
    let Some(element) = find::active() else {
        return;
    };
    let value = element.value();
    let spots = match kind {
        JumpKind::Word => spots(&value, is_word_start),
        JumpKind::Line => spots(&value, is_line_start),
    };
    if spots.is_empty() {
        return;
    }
    let labels = labels(spots.len().min(LIMIT));
    let targets: Vec<JumpTarget> = spots
        .into_iter()
        .zip(labels)
        .map(|(spot, label)| {
            let (x, y) = caret::cell(&element, spot.line, spot.column);
            JumpTarget {
                label,
                x,
                y,
                offset: spot.offset,
            }
        })
        .collect();
    state.jump.set(Some(JumpState {
        targets,
        pending: String::new(),
    }));
}

/// Handles a keystroke while jump mode is active. Returns whether it was used.
pub fn key(state: EditorState, key: &str) -> bool {
    let Some(mut jump) = state.jump.get_untracked() else {
        return false;
    };
    if key == "Escape" {
        state.jump.set(None);
        return true;
    }
    if key.chars().count() != 1 {
        return true;
    }
    let pending = format!("{}{}", jump.pending, key);
    if let Some(target) = jump.targets.iter().find(|target| target.label == pending) {
        let offset = target.offset;
        state.jump.set(None);
        if let Some(element) = find::active() {
            let _ = element.focus();
            let _ = element.set_selection_range(offset, offset);
        }
        return true;
    }
    if jump
        .targets
        .iter()
        .any(|target| target.label.starts_with(&pending))
    {
        jump.pending = pending;
        state.jump.set(Some(jump));
        return true;
    }
    state.jump.set(None);
    true
}

fn labels(count: usize) -> Vec<String> {
    let chars: Vec<char> = LABELS.iter().map(|byte| *byte as char).collect();
    if count <= chars.len() {
        return (0..count).map(|index| chars[index].to_string()).collect();
    }
    let mut out = Vec::with_capacity(count);
    'outer: for first in &chars {
        for second in &chars {
            out.push(format!("{first}{second}"));
            if out.len() >= count {
                break 'outer;
            }
        }
    }
    out
}

fn spots(value: &str, want: impl Fn(Option<char>, char) -> bool) -> Vec<Spot> {
    let mut out = Vec::new();
    let mut line = 0_u32;
    let mut column = 0_u32;
    let mut offset = 0_u32;
    let mut previous: Option<char> = None;
    for character in value.chars() {
        if out.len() >= LIMIT {
            break;
        }
        if want(previous, character) {
            out.push(Spot {
                offset,
                line,
                column,
            });
        }
        let width = character.len_utf16() as u32;
        if character == '\n' {
            line += 1;
            column = 0;
        } else {
            column += width;
        }
        offset += width;
        previous = Some(character);
    }
    out
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn is_word_start(previous: Option<char>, current: char) -> bool {
    is_word(current) && previous.map(|previous| !is_word(previous)).unwrap_or(true)
}

fn is_line_start(previous: Option<char>, current: char) -> bool {
    current != '\n' && previous.map(|previous| previous == '\n').unwrap_or(true)
}
