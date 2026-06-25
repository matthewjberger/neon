//! A buffer-level undo stack. Native textarea undo is bypassed whenever an edit
//! is applied programmatically (editor-plugin ops, find/replace, completion), so
//! this records every committed edit and drives undo/redo itself. Edits within a
//! short window coalesce into one step, the way a typing burst does in a real
//! editor. History is per buffer, so each tab keeps its own.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use crate::components::overlays::find;
use crate::state::{EditorState, PluginKind};

const COALESCE_MS: f64 = 600.0;
const MAX_DEPTH: usize = 300;

#[derive(Default)]
struct History {
    undo: Vec<String>,
    redo: Vec<String>,
    last: f64,
}

thread_local! {
    static HISTORY: RefCell<HashMap<String, History>> = RefCell::new(HashMap::new());
    static APPLYING: Cell<bool> = const { Cell::new(false) };
}

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|performance| performance.now())
        .unwrap_or(0.0)
}

fn key(kind: PluginKind, id: &str) -> String {
    let tag = match kind {
        PluginKind::Scene => "scene",
        PluginKind::Editor => "editor",
        PluginKind::Builtin => "builtin",
        PluginKind::File => "file",
    };
    format!("{tag}:{id}")
}

/// Records the pre-edit text of a buffer. A no-op while undo/redo is applying, so
/// applying a step does not itself become a step.
pub fn record(kind: PluginKind, id: &Option<String>, old_text: &str) {
    if APPLYING.with(Cell::get) {
        return;
    }
    let Some(id) = id else {
        return;
    };
    let key = key(kind, id);
    let now = now_ms();
    HISTORY.with(|history| {
        let mut map = history.borrow_mut();
        let entry = map.entry(key).or_default();
        let burst_start = entry.undo.is_empty() || now - entry.last > COALESCE_MS;
        if burst_start {
            if entry
                .undo
                .last()
                .map(|text| text != old_text)
                .unwrap_or(true)
            {
                entry.undo.push(old_text.to_string());
                if entry.undo.len() > MAX_DEPTH {
                    entry.undo.remove(0);
                }
            }
            entry.redo.clear();
        }
        entry.last = now;
    });
}

/// Restores the previous step for the focused buffer.
pub fn undo(state: EditorState) {
    step(state, true);
}

/// Re-applies the next step for the focused buffer.
pub fn redo(state: EditorState) {
    step(state, false);
}

fn step(state: EditorState, is_undo: bool) {
    let buffer = state.focused_buffer();
    let Some(id) = buffer.id.clone() else {
        return;
    };
    let Some(element) = find::active() else {
        return;
    };
    let key = key(buffer.kind, &id);
    let current = element.value();
    let target = HISTORY.with(|history| {
        let mut map = history.borrow_mut();
        let entry = map.entry(key).or_default();
        let (from, to) = if is_undo {
            (&mut entry.undo, &mut entry.redo)
        } else {
            (&mut entry.redo, &mut entry.undo)
        };
        let target = from.pop()?;
        to.push(current.clone());
        Some(target)
    });
    let Some(target) = target else {
        return;
    };
    let caret = first_divergence(&target, &current);
    APPLYING.with(|applying| applying.set(true));
    element.set_value(&target);
    let _ = element.set_selection_range(caret, caret);
    let _ = element.focus();
    if let Ok(event) = web_sys::Event::new("input") {
        let _ = element.dispatch_event(&event);
    }
    APPLYING.with(|applying| applying.set(false));
}

fn first_divergence(left: &str, right: &str) -> u32 {
    let mut count = 0;
    for (a, b) in left.chars().zip(right.chars()) {
        if a != b {
            break;
        }
        count += a.len_utf16() as u32;
    }
    count
}
