//! The page-side editor-plugin runtime, the Editor API. Editor plugins are rhai
//! that handle keystrokes: `on_key()` reads `key`, `mode`, `ctrl`, `shift`,
//! `alt`, and a persistent `state` map, and pushes ops to `ops`. The host runs
//! every enabled editor plugin, then applies the ops to the code buffer. This
//! mirrors the scene-plugin model, applied to the editor instead of the scene.
//! It is what carries the vim layer.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use leptos::prelude::*;
use rhai::{AST, Array, Dynamic, Engine, Map, Scope};
use web_sys::HtmlTextAreaElement;

use crate::state::{EditorState, LeaderItem, LeaderMenu, PluginKind};

thread_local! {
    static ENGINE: Engine = make_engine();
    static CACHE: RefCell<HashMap<u64, AST>> = RefCell::new(HashMap::new());
    static STATES: RefCell<HashMap<String, Map>> = RefCell::new(HashMap::new());
}

/// A rhai engine with the depth and operation limits lifted, so a plugin with a
/// long key dispatch (like vim) compiles. A bare `Engine::new()` rejects it as
/// exceeding the default expression complexity.
fn make_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(0, 0);
    engine.set_max_operations(0);
    engine
}

/// One editor action a plugin emits.
enum EditorOp {
    Consume,
    SetMode(String),
    SetStatus(String),
    Insert(String),
    Move(i64),
    MoveLine(i64),
    LineStart,
    LineEnd,
    NextWord,
    PrevWord,
    DeleteForward(i64),
    DeleteBackward(i64),
    DeleteLine,
    RunCommand(String),
    OpenPalette,
    ShowMenu(LeaderMenu),
    HideMenu,
}

/// One keystroke handed to the editor plugins.
pub struct KeyEvent {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

/// The result of dispatching a key to the editor plugins.
pub struct KeyOutcome {
    /// Whether a plugin consumed the key, so the browser default is prevented.
    pub consumed: bool,
    /// Whether the buffer text changed, so the caller persists and syncs.
    pub changed: bool,
}

/// Runs every enabled editor plugin's `on_key` for this keystroke and applies
/// the ops to the textarea, returning what happened.
pub fn handle_key(
    state: EditorState,
    id: Option<String>,
    kind: PluginKind,
    textarea: &HtmlTextAreaElement,
    event: &KeyEvent,
) -> KeyOutcome {
    let mode = state.editor_mode.get_untracked();
    let ops = dispatch(state, event, &mode);
    if ops.is_empty() {
        return KeyOutcome {
            consumed: false,
            changed: false,
        };
    }
    let consumed = ops.iter().any(|op| matches!(op, EditorOp::Consume));
    let changed = apply(state, id, kind, textarea, ops);
    KeyOutcome { consumed, changed }
}

fn dispatch(state: EditorState, event: &KeyEvent, mode: &str) -> Vec<EditorOp> {
    let plugins = state.editor_plugins.get_untracked();
    let mut all_ops = Vec::new();
    for plugin in plugins.iter().filter(|plugin| plugin.enabled) {
        let Some(ast) = compiled(&plugin.source) else {
            continue;
        };
        let defines = ast
            .iter_functions()
            .any(|function| function.name == "on_key" && function.params.is_empty());
        if !defines {
            continue;
        }
        let mut scope = Scope::new();
        let plugin_state =
            STATES.with(|states| states.borrow().get(&plugin.id).cloned().unwrap_or_default());
        scope.push("key", event.key.clone());
        scope.push("mode", mode.to_string());
        scope.push("ctrl", event.ctrl);
        scope.push("shift", event.shift);
        scope.push("alt", event.alt);
        scope.push("ops", Array::new());
        scope.push("state", plugin_state);
        let ran = ENGINE
            .with(|engine| engine.call_fn::<()>(&mut scope, &ast, "on_key", ()))
            .is_ok();
        if !ran {
            continue;
        }
        if let Some(updated) = scope.get_value::<Map>("state") {
            STATES.with(|states| {
                states.borrow_mut().insert(plugin.id.clone(), updated);
            });
        }
        if let Some(ops) = scope.get_value::<Array>("ops") {
            for op in ops.iter() {
                if let Some(parsed) = parse_op(op) {
                    all_ops.push(parsed);
                }
            }
        }
    }
    all_ops
}

fn parse_op(value: &Dynamic) -> Option<EditorOp> {
    if let Ok(text) = value.clone().into_string() {
        return match text.as_str() {
            "Consume" => Some(EditorOp::Consume),
            "LineStart" => Some(EditorOp::LineStart),
            "LineEnd" => Some(EditorOp::LineEnd),
            "NextWord" => Some(EditorOp::NextWord),
            "PrevWord" => Some(EditorOp::PrevWord),
            "DeleteLine" => Some(EditorOp::DeleteLine),
            "OpenPalette" => Some(EditorOp::OpenPalette),
            "HideMenu" => Some(EditorOp::HideMenu),
            _ => None,
        };
    }
    let map = value.clone().try_cast::<Map>()?;
    let (name, payload) = map.into_iter().next()?;
    match name.as_str() {
        "ShowMenu" => parse_menu(payload).map(EditorOp::ShowMenu),
        "SetMode" => Some(EditorOp::SetMode(payload.into_string().ok()?)),
        "SetStatus" => Some(EditorOp::SetStatus(payload.into_string().ok()?)),
        "Insert" => Some(EditorOp::Insert(payload.into_string().ok()?)),
        "Move" => Some(EditorOp::Move(payload.as_int().ok()?)),
        "MoveLine" => Some(EditorOp::MoveLine(payload.as_int().ok()?)),
        "DeleteForward" => Some(EditorOp::DeleteForward(payload.as_int().ok()?)),
        "DeleteBackward" => Some(EditorOp::DeleteBackward(payload.as_int().ok()?)),
        "RunCommand" => Some(EditorOp::RunCommand(payload.into_string().ok()?)),
        _ => None,
    }
}

fn parse_menu(payload: Dynamic) -> Option<LeaderMenu> {
    let map = payload.try_cast::<Map>()?;
    let title = map
        .get("title")
        .and_then(|value| value.clone().into_string().ok())
        .unwrap_or_default();
    let items = map
        .get("items")
        .and_then(|value| value.clone().try_cast::<Array>())
        .map(|array| {
            array
                .into_iter()
                .filter_map(|entry| {
                    let entry = entry.try_cast::<Map>()?;
                    let key = entry.get("key")?.clone().into_string().ok()?;
                    let label = entry.get("label")?.clone().into_string().ok()?;
                    Some(LeaderItem { key, label })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(LeaderMenu { title, items })
}

/// Applies the ops to the textarea and the active buffer signal, returning
/// whether the text changed.
fn apply(
    state: EditorState,
    id: Option<String>,
    kind: PluginKind,
    textarea: &HtmlTextAreaElement,
    ops: Vec<EditorOp>,
) -> bool {
    let mut text: Vec<char> = textarea.value().chars().collect();
    let mut caret = textarea.selection_start().ok().flatten().unwrap_or(0) as usize;
    caret = caret.min(text.len());

    let mut changed = false;
    let mut new_mode: Option<String> = None;
    for op in ops {
        match op {
            EditorOp::Consume => {}
            EditorOp::SetMode(mode) => new_mode = Some(mode),
            EditorOp::SetStatus(status) => state.status.set(status),
            EditorOp::Insert(value) => {
                let inserted: Vec<char> = value.chars().collect();
                let count = inserted.len();
                text.splice(caret..caret, inserted);
                caret += count;
                changed = true;
            }
            EditorOp::Move(delta) => caret = shift(caret, delta, text.len()),
            EditorOp::MoveLine(delta) => caret = move_line(&text, caret, delta),
            EditorOp::LineStart => caret = line_start(&text, caret),
            EditorOp::LineEnd => caret = line_end(&text, caret),
            EditorOp::NextWord => caret = next_word(&text, caret),
            EditorOp::PrevWord => caret = prev_word(&text, caret),
            EditorOp::DeleteForward(count) => {
                let end = (caret + count.max(0) as usize).min(text.len());
                if end > caret {
                    text.drain(caret..end);
                    changed = true;
                }
            }
            EditorOp::DeleteBackward(count) => {
                let start = caret.saturating_sub(count.max(0) as usize);
                if caret > start {
                    text.drain(start..caret);
                    caret = start;
                    changed = true;
                }
            }
            EditorOp::DeleteLine => {
                let start = line_start(&text, caret);
                let mut end = line_end(&text, caret);
                if end < text.len() {
                    end += 1;
                }
                if end > start {
                    text.drain(start..end);
                    caret = start.min(text.len());
                    changed = true;
                }
            }
            EditorOp::RunCommand(id) => state.command_request.set(Some(id)),
            EditorOp::OpenPalette => state.palette_open.set(true),
            EditorOp::ShowMenu(menu) => state.leader.set(Some(menu)),
            EditorOp::HideMenu => state.leader.set(None),
        }
    }

    let value: String = text.iter().collect();
    textarea.set_value(&value);
    let caret = caret.min(text.len()) as u32;
    let _ = textarea.set_selection_range(caret, caret);

    if let Some(mode) = new_mode {
        state.editor_mode.set(mode);
    }

    if changed {
        state.editable_set(kind).update(|plugins| {
            if let Some(plugin) = plugins
                .iter_mut()
                .find(|plugin| Some(&plugin.id) == id.as_ref())
            {
                plugin.source = value.clone();
            }
        });
    }
    changed
}

fn shift(caret: usize, delta: i64, len: usize) -> usize {
    let next = caret as i64 + delta;
    next.clamp(0, len as i64) as usize
}

fn line_start(text: &[char], caret: usize) -> usize {
    let mut index = caret.min(text.len());
    while index > 0 && text[index - 1] != '\n' {
        index -= 1;
    }
    index
}

fn line_end(text: &[char], caret: usize) -> usize {
    let mut index = caret.min(text.len());
    while index < text.len() && text[index] != '\n' {
        index += 1;
    }
    index
}

fn move_line(text: &[char], caret: usize, delta: i64) -> usize {
    let column = caret - line_start(text, caret);
    if delta < 0 {
        let start = line_start(text, caret);
        if start == 0 {
            return caret;
        }
        let previous_start = line_start(text, start - 1);
        let previous_end = start - 1;
        (previous_start + column).min(previous_end)
    } else {
        let end = line_end(text, caret);
        if end >= text.len() {
            return caret;
        }
        let next_start = end + 1;
        let next_end = line_end(text, next_start);
        (next_start + column).min(next_end)
    }
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn next_word(text: &[char], caret: usize) -> usize {
    let mut index = caret;
    while index < text.len() && is_word(text[index]) {
        index += 1;
    }
    while index < text.len() && !is_word(text[index]) {
        index += 1;
    }
    index
}

fn prev_word(text: &[char], caret: usize) -> usize {
    let mut index = caret;
    while index > 0 && !is_word(text[index - 1]) {
        index -= 1;
    }
    while index > 0 && is_word(text[index - 1]) {
        index -= 1;
    }
    index
}

fn compiled(source: &str) -> Option<AST> {
    let key = hash_source(source);
    CACHE.with(|cache| {
        if let Some(ast) = cache.borrow().get(&key) {
            return Some(ast.clone());
        }
        let ast = ENGINE.with(|engine| engine.compile(source).ok())?;
        cache.borrow_mut().insert(key, ast.clone());
        Some(ast)
    })
}

fn hash_source(source: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

/// Inserts text at the caret and updates the active buffer. Used for the editor's
/// own Tab-to-indent, independent of any plugin.
pub fn insert_text(
    state: EditorState,
    id: Option<String>,
    kind: PluginKind,
    textarea: &HtmlTextAreaElement,
    text: &str,
) {
    let mut chars: Vec<char> = textarea.value().chars().collect();
    let mut caret = textarea.selection_start().ok().flatten().unwrap_or(0) as usize;
    caret = caret.min(chars.len());
    let inserted: Vec<char> = text.chars().collect();
    let count = inserted.len();
    chars.splice(caret..caret, inserted);
    let value: String = chars.iter().collect();
    textarea.set_value(&value);
    let caret = (caret + count) as u32;
    let _ = textarea.set_selection_range(caret, caret);
    state.editable_set(kind).update(|plugins| {
        if let Some(plugin) = plugins
            .iter_mut()
            .find(|plugin| Some(&plugin.id) == id.as_ref())
        {
            plugin.source = value.clone();
        }
    });
}

/// Whether any editor plugin is enabled, so the editor pane should route keys.
pub fn any_enabled(state: EditorState) -> bool {
    state
        .editor_plugins
        .get_untracked()
        .iter()
        .any(|plugin| plugin.enabled)
}

/// Resets the editor mode to normal when entering or leaving editor plugins.
pub fn reset_mode(state: EditorState) {
    state.editor_mode.set("normal".to_string());
}
