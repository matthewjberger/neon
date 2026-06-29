//! A buffer-level undo *tree*. Native textarea undo is bypassed whenever an edit
//! is applied programmatically (editor-plugin ops, find/replace, completion), so
//! this records every committed state and drives undo/redo itself. History
//! branches: undoing and then editing keeps the old future as a sibling branch
//! rather than discarding it, the way undo-tree and Gundo do. Edits within a
//! short window coalesce into one node. History is per buffer.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use crate::components::overlays::find;
use crate::state::{EditorState, PluginKind};

const COALESCE_MS: f64 = 600.0;

/// One state in a buffer's history: its full text, its parent, and the branches
/// off it.
#[derive(Clone)]
struct Node {
    text: String,
    parent: Option<usize>,
    children: Vec<usize>,
}

/// One buffer's history tree: the nodes, the live node, the last edit time, and
/// whether the live node is still being typed into (so a burst coalesces).
struct History {
    nodes: Vec<Node>,
    current: usize,
    last: f64,
    fresh: bool,
}

impl History {
    fn new(text: &str, time: f64) -> Self {
        Self {
            nodes: vec![Node {
                text: text.to_string(),
                parent: None,
                children: Vec::new(),
            }],
            current: 0,
            last: time,
            fresh: false,
        }
    }
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

/// Commits an edit: the buffer went from `old_text` to `new_text`. A no-op while
/// undo/redo is applying. A burst coalesces into the live node; otherwise a new
/// node branches off the current one, keeping any existing branches.
pub fn record(kind: PluginKind, id: &Option<String>, old_text: &str, new_text: &str) {
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
        let entry = map
            .entry(key)
            .or_insert_with(|| History::new(old_text, now));
        if entry.fresh && now - entry.last <= COALESCE_MS {
            let current = entry.current;
            entry.nodes[current].text = new_text.to_string();
        } else {
            let index = entry.nodes.len();
            let parent = entry.current;
            entry.nodes.push(Node {
                text: new_text.to_string(),
                parent: Some(parent),
                children: Vec::new(),
            });
            entry.nodes[parent].children.push(index);
            entry.current = index;
            entry.fresh = true;
        }
        entry.last = now;
    });
}

/// Restores the parent of the live node for the focused buffer.
pub fn undo(state: EditorState) {
    navigate(state, |entry| entry.nodes[entry.current].parent);
}

/// Re-applies the most recent branch off the live node.
pub fn redo(state: EditorState) {
    navigate(state, |entry| {
        entry.nodes[entry.current].children.last().copied()
    });
}

/// Moves the live node to whichever node `pick` returns and restores its text.
fn navigate(state: EditorState, pick: impl FnOnce(&History) -> Option<usize>) {
    let buffer = state.focused_buffer();
    let Some(id) = buffer.id.clone() else {
        return;
    };
    let key = key(buffer.kind, &id);
    let target = HISTORY.with(|history| {
        let mut map = history.borrow_mut();
        let entry = map.get_mut(&key)?;
        let next = pick(entry)?;
        entry.current = next;
        entry.fresh = false;
        Some(entry.nodes[next].text.clone())
    });
    if let Some(target) = target {
        restore_text(&target);
    }
}

/// Jumps directly to a node by index, the click target of the visualizer.
pub fn restore(state: EditorState, index: usize) {
    let buffer = state.focused_buffer();
    let Some(id) = buffer.id.clone() else {
        return;
    };
    let key = key(buffer.kind, &id);
    let target = HISTORY.with(|history| {
        let mut map = history.borrow_mut();
        let entry = map.get_mut(&key)?;
        let node = entry.nodes.get(index)?;
        let text = node.text.clone();
        entry.current = index;
        entry.fresh = false;
        Some(text)
    });
    if let Some(target) = target {
        restore_text(&target);
    }
}

/// One row of the visualizer: a node's depth, a text preview, and whether it is
/// the live node.
#[derive(Clone, PartialEq)]
pub struct UndoRow {
    pub index: usize,
    pub depth: usize,
    pub preview: String,
    pub current: bool,
}

/// The focused buffer's history as a depth-indented preorder list, newest branch
/// first, for the visualizer panel.
pub fn rows(state: EditorState) -> Vec<UndoRow> {
    let buffer = state.focused_buffer();
    let Some(id) = buffer.id else {
        return Vec::new();
    };
    let key = key(buffer.kind, &id);
    HISTORY.with(|history| {
        let map = history.borrow();
        let Some(entry) = map.get(&key) else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        let mut stack = vec![(0_usize, 0_usize)];
        while let Some((index, depth)) = stack.pop() {
            let Some(node) = entry.nodes.get(index) else {
                continue;
            };
            rows.push(UndoRow {
                index,
                depth,
                preview: preview(&node.text),
                current: index == entry.current,
            });
            for child in node.children.iter().rev() {
                stack.push((*child, depth + 1));
            }
        }
        rows
    })
}

/// A one-line preview of a state: its first non-blank line, trimmed.
fn preview(text: &str) -> String {
    let line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();
    let snippet: String = line.chars().take(40).collect();
    if snippet.is_empty() {
        format!("{} chars", text.chars().count())
    } else {
        snippet
    }
}

fn restore_text(target: &str) {
    let Some(element) = find::active() else {
        return;
    };
    let current = element.value();
    let caret = first_divergence(target, &current);
    APPLYING.with(|applying| applying.set(true));
    element.set_value(target);
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
