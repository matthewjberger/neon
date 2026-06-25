//! Avy-style jump: label every word start, line start, or occurrence of a
//! character that is on screen, show the labels over a dimmed buffer, and jump
//! the caret to the one whose label you type. Spacemacs' `SPC j w`, `SPC j l`,
//! and `SPC j j`. The labels are the sentinels: type a label to jump, Escape to
//! cancel.

use leptos::prelude::*;
use web_sys::HtmlTextAreaElement;

use crate::caret;
use crate::components::overlays::find;
use crate::state::{EditorState, JumpState, JumpTarget};

const LABELS: &[u8] = b"asdfghjklqwertyuiopzxcvbnm";
const LIMIT: usize = 200;

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

/// Enters jump mode for the focused buffer, labeling word or line starts on
/// screen.
pub fn start(state: EditorState, kind: JumpKind) {
    let Some(element) = find::active() else {
        return;
    };
    let value = element.value();
    let (first, last) = visible_lines(&element);
    let spots = match kind {
        JumpKind::Word => spots(&value, first, last, is_word_start),
        JumpKind::Line => spots(&value, first, last, is_line_start),
    };
    show(state, &element, spots);
}

/// Enters jump-to-character mode: the next keystroke chooses the character.
pub fn start_char(state: EditorState) {
    if find::active().is_none() {
        return;
    }
    state.editing.jump.set(Some(JumpState {
        targets: Vec::new(),
        pending: String::new(),
        awaiting_char: true,
    }));
}

/// Handles a keystroke while jump mode is active. Returns whether it was used.
pub fn key(state: EditorState, key: &str) -> bool {
    let Some(mut jump) = state.editing.jump.get_untracked() else {
        return false;
    };
    if key == "Escape" {
        state.editing.jump.set(None);
        return true;
    }
    if key.chars().count() != 1 {
        return true;
    }
    if jump.awaiting_char {
        let needle = key.chars().next().unwrap();
        let Some(element) = find::active() else {
            state.editing.jump.set(None);
            return true;
        };
        let value = element.value();
        let (first, last) = visible_lines(&element);
        let spots = spots(&value, first, last, move |_, current| current == needle);
        show(state, &element, spots);
        return true;
    }
    let pending = format!("{}{}", jump.pending, key);
    if let Some(target) = jump.targets.iter().find(|target| target.label == pending) {
        let offset = target.offset;
        state.editing.jump.set(None);
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
        state.editing.jump.set(Some(jump));
        return true;
    }
    state.editing.jump.set(None);
    true
}

fn show(state: EditorState, element: &HtmlTextAreaElement, spots: Vec<Spot>) {
    if spots.is_empty() {
        state.editing.jump.set(None);
        return;
    }
    let labels = labels(spots.len());
    let targets: Vec<JumpTarget> = spots
        .into_iter()
        .zip(labels)
        .map(|(spot, label)| {
            let (x, y) = caret::cell(element, spot.line, spot.column);
            JumpTarget {
                label,
                x,
                y,
                offset: spot.offset,
            }
        })
        .collect();
    state.editing.jump.set(Some(JumpState {
        targets,
        pending: String::new(),
        awaiting_char: false,
    }));
}

/// The inclusive range of document lines currently scrolled into view. Falls
/// back to the whole buffer (capped by `LIMIT`) when the pane has no measured
/// height yet, so a jump right after layout still finds targets.
fn visible_lines(element: &HtmlTextAreaElement) -> (u32, u32) {
    let height = element.client_height() as f64;
    if height <= 0.0 {
        return (0, u32::MAX);
    }
    let line_height = caret::line_height(element).max(1.0);
    let top = element.scroll_top() as f64;
    let first = (top / line_height).floor().max(0.0) as u32;
    let last = ((top + height) / line_height).ceil() as u32 + 1;
    (first, last)
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

fn spots(
    value: &str,
    first_line: u32,
    last_line: u32,
    want: impl Fn(Option<char>, char) -> bool,
) -> Vec<Spot> {
    let mut out = Vec::new();
    let mut line = 0_u32;
    let mut column = 0_u32;
    let mut offset = 0_u32;
    let mut previous: Option<char> = None;
    for character in value.chars() {
        if line > last_line || out.len() >= LIMIT {
            break;
        }
        if line >= first_line && want(previous, character) {
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
