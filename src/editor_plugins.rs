//! The page-side editor-plugin runtime, the Editor API. Editor plugins are rhai
//! that handle keystrokes: `on_key()` reads `key`, `mode`, `ctrl`, `shift`,
//! `alt`, and a persistent `state` map, and pushes ops to `ops`. The host runs
//! every enabled editor plugin, then applies the ops to the code buffer. This
//! mirrors the scene-plugin model, applied to the editor instead of the scene.
//! It is what carries the vim layer.

use std::cell::{Cell, RefCell};
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
    /// The region anchor (a char index), set by `Anchor` and cleared by
    /// `Collapse` or any region-consuming edit. While it is set, motion ops
    /// extend the textarea selection from it to the caret, which is how visual
    /// mode works without the plugin tracking the selection itself.
    static MARK: Cell<Option<usize>> = const { Cell::new(None) };
    /// The kill-ring: every `Copy`/`Cut` pushes here, `Paste` reads the top.
    static KILL_RING: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// Named registers, written by a `Register` op before a yank or paste.
    static REGISTERS: RefCell<HashMap<char, String>> = RefCell::new(HashMap::new());
    /// The register a `Register` op named, consumed by the next yank or paste.
    static PENDING_REGISTER: Cell<Option<char>> = const { Cell::new(None) };
    /// The last charwise paste `(start, length, ring index)`, so `PastePop` can
    /// swap it for the previous ring entry, an Emacs-style yank-pop.
    static LAST_PASTE: Cell<Option<(usize, usize, usize)>> = const { Cell::new(None) };
    /// The last buffer search `(needle, forward)`, repeated by `SearchNext` and
    /// `SearchPrev`.
    static LAST_SEARCH: RefCell<Option<(String, bool)>> = const { RefCell::new(None) };
    /// The last in-line find `(char, kind)`, repeated by `RepeatFind`.
    static LAST_FIND: Cell<Option<(char, FindKind)>> = const { Cell::new(None) };
    /// The op batch of the last buffer change, replayed by `Repeat` (the dot).
    static LAST_CHANGE: RefCell<Vec<EditorOp>> = const { RefCell::new(Vec::new()) };
}

/// How an in-line find lands: on the character (`f`/`F`) or just before/after it
/// (`t`/`T`), forward or backward.
#[derive(Clone, Copy, PartialEq)]
enum FindKind {
    Find,
    FindBack,
    Till,
    TillBack,
}

/// The most entries the kill-ring keeps before the oldest falls off.
const KILL_RING_LIMIT: usize = 32;

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
#[derive(Clone)]
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
    DeleteToLineEnd,
    DeleteWordBackward,
    DeleteWordForward,
    DuplicateLine,
    MoveLineUp,
    MoveLineDown,
    JoinLines,
    Indent,
    Outdent,
    SmartLineStart,
    UpperCaseWord,
    LowerCaseWord,
    SortLines,
    DeleteTrailingWhitespace,
    ToggleComment(String),
    FindChar(String),
    RunCommand(String),
    OpenPalette,
    ShowMenu(LeaderMenu),
    HideMenu,
    /// Region: start a selection at the caret.
    Anchor,
    /// Region: clear the selection, collapsing to the caret.
    Collapse,
    /// Region: move the caret to the other end of the selection.
    SwapEnds,
    /// Region: select the current line.
    SelectLine,
    /// Region: select the word under the caret.
    SelectWord,
    /// Region: select the whole buffer.
    SelectAll,
    /// Region: select inside the nearest pair named by the spec (a bracket, a
    /// quote, or `w` for the word), the inner text object.
    SelectInner(String),
    /// Region: like `SelectInner` but include the delimiters, the around object.
    SelectAround(String),
    /// Region: delete the selection.
    DeleteSelection,
    /// Region: replace the selection with text.
    ReplaceSelection(String),
    /// Region: wrap the selection (or the word under the caret when there is no
    /// selection) with an opening and closing string.
    Surround(String, String),
    /// Kill-ring: copy the selection, or the current line when there is none.
    Copy,
    /// Kill-ring: copy then delete the selection.
    Cut,
    /// Kill-ring: insert the top of the ring (or the named register) at the caret.
    Paste,
    /// Kill-ring: replace the last paste with the previous ring entry.
    PastePop,
    /// Kill-ring: name the register the next yank or paste uses.
    Register(String),
    /// Search: move to the next match of a needle in the buffer, forward.
    Search(String),
    /// Search: like `Search`, backward.
    SearchBack(String),
    /// Search: repeat the last search in its own direction.
    SearchNext,
    /// Search: repeat the last search in the opposite direction.
    SearchPrev,
    /// In-line find: move backward to a character on the line (`F`).
    FindCharBack(String),
    /// In-line find: move forward to just before a character (`t`).
    TillChar(String),
    /// In-line find: move backward to just after a character (`T`).
    TillCharBack(String),
    /// In-line find: repeat the last find in its direction (`;`).
    RepeatFind,
    /// In-line find: repeat the last find in the opposite direction (`,`).
    RepeatFindBack,
    /// Replay the last buffer change (`.`).
    Repeat,
    /// Move the caret to the start of a 1-based line.
    Goto(i64),
    /// Move the caret to the start of the buffer (`gg`).
    BufferStart,
    /// Move the caret to the last line (`G`).
    BufferEnd,
    /// Scroll so the caret line sits at the middle of the view (`zz`).
    Center,
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
    let mode = state.editing.mode.get_untracked();
    let introspection = introspect(textarea);
    let mut ops = dispatch(state, event, &mode, &introspection);
    if ops.is_empty() {
        return KeyOutcome {
            consumed: false,
            changed: false,
        };
    }
    // A dot replays the recorded change rather than its own batch, and is never
    // itself recorded, so repeating a repeat repeats the original change.
    let is_repeat = ops.iter().any(|op| matches!(op, EditorOp::Repeat));
    if is_repeat {
        ops = LAST_CHANGE.with(|cell| cell.borrow().clone());
        if ops.is_empty() {
            return KeyOutcome {
                consumed: true,
                changed: false,
            };
        }
    }
    let consumed = is_repeat || ops.iter().any(|op| matches!(op, EditorOp::Consume));
    let record = if is_repeat { None } else { Some(ops.clone()) };
    let changed = apply(state, id, kind, textarea, ops);
    if changed && let Some(batch) = record {
        LAST_CHANGE.with(|cell| *cell.borrow_mut() = batch);
    }
    KeyOutcome { consumed, changed }
}

/// The read-only view of the buffer a plugin's `on_key` sees: where the caret is,
/// the line it sits on, the selected text, and the word under it. The host owns
/// the buffer, so this is how a plugin reasons about content (text objects,
/// surround, search the word under the caret) without touching the textarea.
struct Introspection {
    caret_line: i64,
    caret_column: i64,
    caret_offset: i64,
    line_text: String,
    selection: String,
    word: String,
}

/// Reads the introspection a keystroke exposes from the focused textarea.
fn introspect(textarea: &HtmlTextAreaElement) -> Introspection {
    let value = textarea.value();
    let chars: Vec<char> = value.chars().collect();
    let selection_start = textarea.selection_start().ok().flatten().unwrap_or(0) as usize;
    let selection_end = textarea.selection_end().ok().flatten().unwrap_or(0) as usize;
    let caret = selection_start.min(chars.len());
    let start = line_start(&chars, caret);
    let end = line_end(&chars, caret);
    let caret_line = chars[..start]
        .iter()
        .filter(|character| **character == '\n')
        .count() as i64;
    let line_text: String = chars[start..end].iter().collect();
    let mut word_start = caret;
    while word_start > 0 && is_word(chars[word_start - 1]) {
        word_start -= 1;
    }
    let mut word_end = caret;
    while word_end < chars.len() && is_word(chars[word_end]) {
        word_end += 1;
    }
    let word: String = chars[word_start..word_end].iter().collect();
    Introspection {
        caret_line,
        caret_column: (caret - start) as i64,
        caret_offset: caret as i64,
        line_text,
        selection: selection_text(&value, selection_start, selection_end),
        word,
    }
}

/// The selected text between two UTF-16 offsets, or empty when nothing is
/// selected.
fn selection_text(value: &str, start: usize, end: usize) -> String {
    if end <= start {
        return String::new();
    }
    let units: Vec<u16> = value.encode_utf16().collect();
    if start >= units.len() {
        return String::new();
    }
    String::from_utf16_lossy(&units[start..end.min(units.len())])
}

/// The caret introspection as the rhai map a plugin reads as `caret.line`,
/// `caret.column`, and `caret.offset`.
fn caret_map(introspection: &Introspection) -> Map {
    let mut map = Map::new();
    map.insert("line".into(), Dynamic::from_int(introspection.caret_line));
    map.insert(
        "column".into(),
        Dynamic::from_int(introspection.caret_column),
    );
    map.insert(
        "offset".into(),
        Dynamic::from_int(introspection.caret_offset),
    );
    map
}

fn dispatch(
    state: EditorState,
    event: &KeyEvent,
    mode: &str,
    introspection: &Introspection,
) -> Vec<EditorOp> {
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
        scope.push("caret", caret_map(introspection));
        scope.push("line_text", introspection.line_text.clone());
        scope.push("selection", introspection.selection.clone());
        scope.push("word", introspection.word.clone());
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
            "DeleteToLineEnd" => Some(EditorOp::DeleteToLineEnd),
            "DeleteWordBackward" => Some(EditorOp::DeleteWordBackward),
            "DeleteWordForward" => Some(EditorOp::DeleteWordForward),
            "DuplicateLine" => Some(EditorOp::DuplicateLine),
            "MoveLineUp" => Some(EditorOp::MoveLineUp),
            "MoveLineDown" => Some(EditorOp::MoveLineDown),
            "JoinLines" => Some(EditorOp::JoinLines),
            "Indent" => Some(EditorOp::Indent),
            "Outdent" => Some(EditorOp::Outdent),
            "SmartLineStart" => Some(EditorOp::SmartLineStart),
            "UpperCaseWord" => Some(EditorOp::UpperCaseWord),
            "LowerCaseWord" => Some(EditorOp::LowerCaseWord),
            "SortLines" => Some(EditorOp::SortLines),
            "DeleteTrailingWhitespace" => Some(EditorOp::DeleteTrailingWhitespace),
            "OpenPalette" => Some(EditorOp::OpenPalette),
            "HideMenu" => Some(EditorOp::HideMenu),
            "Anchor" => Some(EditorOp::Anchor),
            "Collapse" => Some(EditorOp::Collapse),
            "SwapEnds" => Some(EditorOp::SwapEnds),
            "SelectLine" => Some(EditorOp::SelectLine),
            "SelectWord" => Some(EditorOp::SelectWord),
            "SelectAll" => Some(EditorOp::SelectAll),
            "DeleteSelection" => Some(EditorOp::DeleteSelection),
            "Copy" => Some(EditorOp::Copy),
            "Cut" => Some(EditorOp::Cut),
            "Paste" => Some(EditorOp::Paste),
            "PastePop" => Some(EditorOp::PastePop),
            "SearchNext" => Some(EditorOp::SearchNext),
            "SearchPrev" => Some(EditorOp::SearchPrev),
            "RepeatFind" => Some(EditorOp::RepeatFind),
            "RepeatFindBack" => Some(EditorOp::RepeatFindBack),
            "Repeat" => Some(EditorOp::Repeat),
            "BufferStart" => Some(EditorOp::BufferStart),
            "BufferEnd" => Some(EditorOp::BufferEnd),
            "Center" => Some(EditorOp::Center),
            _ => None,
        };
    }
    let map = value.clone().try_cast::<Map>()?;
    let (name, payload) = map.into_iter().next()?;
    match name.as_str() {
        "ShowMenu" => parse_menu(payload).map(EditorOp::ShowMenu),
        "SelectInner" => Some(EditorOp::SelectInner(payload.into_string().ok()?)),
        "SelectAround" => Some(EditorOp::SelectAround(payload.into_string().ok()?)),
        "ReplaceSelection" => Some(EditorOp::ReplaceSelection(payload.into_string().ok()?)),
        "Register" => Some(EditorOp::Register(payload.into_string().ok()?)),
        "Search" => Some(EditorOp::Search(payload.into_string().ok()?)),
        "SearchBack" => Some(EditorOp::SearchBack(payload.into_string().ok()?)),
        "Goto" => Some(EditorOp::Goto(payload.as_int().ok()?)),
        "FindCharBack" => Some(EditorOp::FindCharBack(payload.into_string().ok()?)),
        "TillChar" => Some(EditorOp::TillChar(payload.into_string().ok()?)),
        "TillCharBack" => Some(EditorOp::TillCharBack(payload.into_string().ok()?)),
        "Surround" => {
            let pair = payload.try_cast::<Array>()?;
            let open = pair.first()?.clone().into_string().ok()?;
            let close = pair.get(1)?.clone().into_string().ok()?;
            Some(EditorOp::Surround(open, close))
        }
        "ToggleComment" => Some(EditorOp::ToggleComment(payload.into_string().ok()?)),
        "FindChar" => Some(EditorOp::FindChar(payload.into_string().ok()?)),
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
    let mut mark = MARK.with(Cell::get).map(|index| index.min(text.len()));

    let mut changed = false;
    let mut new_mode: Option<String> = None;
    for op in ops {
        match op {
            EditorOp::Consume => {}
            EditorOp::SetMode(mode) => new_mode = Some(mode),
            EditorOp::SetStatus(status) => state.editing.status.set(status),
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
            EditorOp::DeleteToLineEnd => {
                let end = line_end(&text, caret);
                if end > caret {
                    text.drain(caret..end);
                    changed = true;
                } else if caret < text.len() && text[caret] == '\n' {
                    text.remove(caret);
                    changed = true;
                }
            }
            EditorOp::DeleteWordBackward => {
                let start = prev_word(&text, caret);
                if caret > start {
                    text.drain(start..caret);
                    caret = start;
                    changed = true;
                }
            }
            EditorOp::DeleteWordForward => {
                let end = next_word(&text, caret);
                if end > caret {
                    text.drain(caret..end);
                    changed = true;
                }
            }
            EditorOp::DuplicateLine => {
                let start = line_start(&text, caret);
                let end = line_end(&text, caret);
                let mut inserted = vec!['\n'];
                inserted.extend_from_slice(&text[start..end]);
                text.splice(end..end, inserted);
                caret += end - start + 1;
                changed = true;
            }
            EditorOp::MoveLineUp => {
                let start = line_start(&text, caret);
                if start > 0 {
                    let end = line_end(&text, caret);
                    let previous_start = line_start(&text, start - 1);
                    let column = caret - start;
                    let mut region: Vec<char> = text[start..end].to_vec();
                    region.push('\n');
                    region.extend_from_slice(&text[previous_start..start - 1]);
                    text.splice(previous_start..end, region);
                    caret = previous_start + column;
                    changed = true;
                }
            }
            EditorOp::MoveLineDown => {
                let end = line_end(&text, caret);
                if end < text.len() {
                    let start = line_start(&text, caret);
                    let next_start = end + 1;
                    let next_end = line_end(&text, next_start);
                    let column = caret - start;
                    let next_line: Vec<char> = text[next_start..next_end].to_vec();
                    let mut region = next_line.clone();
                    region.push('\n');
                    region.extend_from_slice(&text[start..end]);
                    text.splice(start..next_end, region);
                    caret = start + next_line.len() + 1 + column;
                    changed = true;
                }
            }
            EditorOp::JoinLines => {
                let end = line_end(&text, caret);
                if end < text.len() {
                    text.remove(end);
                    let mut whitespace = 0;
                    while end + whitespace < text.len()
                        && (text[end + whitespace] == ' ' || text[end + whitespace] == '\t')
                    {
                        whitespace += 1;
                    }
                    if whitespace > 0 {
                        text.drain(end..end + whitespace);
                    }
                    let previous_is_break = end == 0
                        || text[end - 1] == ' '
                        || text[end - 1] == '\t'
                        || text[end - 1] == '\n';
                    let has_following = end < text.len() && text[end] != '\n';
                    if !previous_is_break && has_following {
                        text.insert(end, ' ');
                    }
                    caret = end;
                    changed = true;
                }
            }
            EditorOp::Indent => {
                if let Some((start, end)) = selection_range(mark, caret) {
                    caret = map_block_lines(&mut text, start, end, |line| {
                        if line.trim().is_empty() {
                            line.to_string()
                        } else {
                            format!("    {line}")
                        }
                    });
                    mark = None;
                } else {
                    let start = line_start(&text, caret);
                    text.splice(start..start, [' ', ' ', ' ', ' ']);
                    caret += 4;
                }
                changed = true;
            }
            EditorOp::Outdent => {
                if let Some((start, end)) = selection_range(mark, caret) {
                    caret = map_block_lines(&mut text, start, end, outdent_line);
                    mark = None;
                    changed = true;
                } else {
                    let start = line_start(&text, caret);
                    let mut removed = 0;
                    while removed < 4
                        && start + removed < text.len()
                        && text[start + removed] == ' '
                    {
                        removed += 1;
                    }
                    if removed > 0 {
                        text.drain(start..start + removed);
                        caret = caret.saturating_sub(removed);
                        changed = true;
                    }
                }
            }
            EditorOp::SmartLineStart => {
                let start = line_start(&text, caret);
                let end = line_end(&text, caret);
                let mut first = start;
                while first < end && (text[first] == ' ' || text[first] == '\t') {
                    first += 1;
                }
                caret = if caret == first { start } else { first };
            }
            EditorOp::UpperCaseWord | EditorOp::LowerCaseWord => {
                let upper = matches!(op, EditorOp::UpperCaseWord);
                let (start, end) =
                    selection_range(mark, caret).unwrap_or_else(|| word_bounds(&text, caret));
                if end > start {
                    for character in text.iter_mut().take(end).skip(start) {
                        *character = if upper {
                            character.to_ascii_uppercase()
                        } else {
                            character.to_ascii_lowercase()
                        };
                    }
                    if mark.is_some() {
                        caret = start;
                        mark = None;
                    }
                    changed = true;
                }
            }
            EditorOp::SortLines => {
                let content: String = text.iter().collect();
                let mut lines: Vec<&str> = content.split('\n').collect();
                lines.sort_unstable();
                let sorted = lines.join("\n");
                if sorted != content {
                    text = sorted.chars().collect();
                    caret = caret.min(text.len());
                    changed = true;
                }
            }
            EditorOp::DeleteTrailingWhitespace => {
                let content: String = text.iter().collect();
                let trimmed = content
                    .split('\n')
                    .map(|line| line.trim_end_matches([' ', '\t']))
                    .collect::<Vec<_>>()
                    .join("\n");
                if trimmed != content {
                    text = trimmed.chars().collect();
                    caret = caret.min(text.len());
                    changed = true;
                }
            }
            EditorOp::ToggleComment(marker) => {
                if let Some((start, end)) = selection_range(mark, caret) {
                    if !marker.is_empty() {
                        caret = comment_block(&mut text, start, end, &marker);
                        changed = true;
                    }
                    mark = None;
                    continue;
                }
                let marker: Vec<char> = marker.chars().collect();
                if !marker.is_empty() {
                    let start = line_start(&text, caret);
                    let end = line_end(&text, caret);
                    let mut first = start;
                    while first < end && (text[first] == ' ' || text[first] == '\t') {
                        first += 1;
                    }
                    let present = first + marker.len() <= end
                        && text[first..first + marker.len()] == marker[..];
                    if present {
                        let mut stop = first + marker.len();
                        if stop < end && text[stop] == ' ' {
                            stop += 1;
                        }
                        let removed = stop - first;
                        text.drain(first..stop);
                        if caret >= stop {
                            caret -= removed;
                        } else if caret > first {
                            caret = first;
                        }
                    } else {
                        let mut inserted = marker.clone();
                        inserted.push(' ');
                        let added = inserted.len();
                        text.splice(first..first, inserted);
                        if caret >= first {
                            caret += added;
                        }
                    }
                    changed = true;
                }
            }
            EditorOp::FindChar(target) => find_char(&mut caret, &text, &target, FindKind::Find),
            EditorOp::FindCharBack(target) => {
                find_char(&mut caret, &text, &target, FindKind::FindBack)
            }
            EditorOp::TillChar(target) => find_char(&mut caret, &text, &target, FindKind::Till),
            EditorOp::TillCharBack(target) => {
                find_char(&mut caret, &text, &target, FindKind::TillBack)
            }
            EditorOp::RepeatFind => {
                if let Some((needle, kind)) = LAST_FIND.with(Cell::get)
                    && let Some(index) = find_on_line(&text, caret, needle, kind)
                {
                    caret = index;
                }
            }
            EditorOp::RepeatFindBack => {
                if let Some((needle, kind)) = LAST_FIND.with(Cell::get)
                    && let Some(index) = find_on_line(&text, caret, needle, opposite_find(kind))
                {
                    caret = index;
                }
            }
            EditorOp::Search(needle) => {
                let chars: Vec<char> = needle.chars().collect();
                if let Some(index) = find_match(&text, caret, &chars, true) {
                    caret = index;
                }
                LAST_SEARCH.with(|cell| *cell.borrow_mut() = Some((needle, true)));
            }
            EditorOp::SearchBack(needle) => {
                let chars: Vec<char> = needle.chars().collect();
                if let Some(index) = find_match(&text, caret, &chars, false) {
                    caret = index;
                }
                LAST_SEARCH.with(|cell| *cell.borrow_mut() = Some((needle, false)));
            }
            EditorOp::SearchNext => {
                if let Some((needle, forward)) = LAST_SEARCH.with(|cell| cell.borrow().clone()) {
                    let chars: Vec<char> = needle.chars().collect();
                    if let Some(index) = find_match(&text, caret, &chars, forward) {
                        caret = index;
                    }
                }
            }
            EditorOp::SearchPrev => {
                if let Some((needle, forward)) = LAST_SEARCH.with(|cell| cell.borrow().clone()) {
                    let chars: Vec<char> = needle.chars().collect();
                    if let Some(index) = find_match(&text, caret, &chars, !forward) {
                        caret = index;
                    }
                }
            }
            EditorOp::Repeat => {}
            EditorOp::Goto(line) => {
                let target = (line.max(1) - 1) as usize;
                let mut offset = 0;
                let mut current = 0;
                while offset < text.len() && current < target {
                    if text[offset] == '\n' {
                        current += 1;
                    }
                    offset += 1;
                }
                caret = offset.min(text.len());
            }
            EditorOp::BufferStart => caret = 0,
            EditorOp::BufferEnd => caret = line_start(&text, text.len()),
            EditorOp::Center => {
                let line_index = text[..caret.min(text.len())]
                    .iter()
                    .filter(|character| **character == '\n')
                    .count();
                let line_height = crate::caret::line_height(textarea).max(1.0);
                let view = textarea.client_height() as f64;
                let target = (line_index as f64) * line_height - view / 2.0;
                textarea.set_scroll_top(target.max(0.0) as i32);
            }
            EditorOp::RunCommand(id) => state.editing.command_request.set(Some(id)),
            EditorOp::OpenPalette => state.editing.palette_open.set(true),
            EditorOp::ShowMenu(menu) => state.editing.leader.set(Some(menu)),
            EditorOp::HideMenu => state.editing.leader.set(None),
            EditorOp::Anchor => mark = Some(caret),
            EditorOp::Collapse => mark = None,
            EditorOp::SwapEnds => {
                if let Some(anchor) = mark {
                    mark = Some(caret);
                    caret = anchor;
                }
            }
            EditorOp::SelectLine => {
                mark = Some(line_start(&text, caret));
                caret = line_end(&text, caret);
            }
            EditorOp::SelectWord => {
                let (start, end) = word_bounds(&text, caret);
                mark = Some(start);
                caret = end;
            }
            EditorOp::SelectAll => {
                mark = Some(0);
                caret = text.len();
            }
            EditorOp::SelectInner(spec) => {
                if let Some((start, end)) = object_bounds(&text, caret, &spec, false) {
                    mark = Some(start);
                    caret = end;
                }
            }
            EditorOp::SelectAround(spec) => {
                if let Some((start, end)) = object_bounds(&text, caret, &spec, true) {
                    mark = Some(start);
                    caret = end;
                }
            }
            EditorOp::DeleteSelection => {
                if let Some((start, end)) = selection_range(mark, caret) {
                    if end > start {
                        text.drain(start..end);
                        caret = start;
                        changed = true;
                    }
                    mark = None;
                }
            }
            EditorOp::ReplaceSelection(value) => {
                if let Some((start, end)) = selection_range(mark, caret) {
                    let inserted: Vec<char> = value.chars().collect();
                    let count = inserted.len();
                    text.splice(start..end, inserted);
                    caret = start + count;
                    changed = true;
                    mark = None;
                }
            }
            EditorOp::Surround(open, close) => {
                let (start, end) =
                    selection_range(mark, caret).unwrap_or_else(|| word_bounds(&text, caret));
                let open_chars: Vec<char> = open.chars().collect();
                let open_count = open_chars.len();
                text.splice(end..end, close.chars());
                text.splice(start..start, open_chars);
                caret = start + open_count;
                changed = true;
                mark = None;
            }
            EditorOp::Register(name) => {
                PENDING_REGISTER.with(|cell| cell.set(name.chars().next()));
            }
            EditorOp::Copy => {
                let yanked = match selection_range(mark, caret) {
                    Some((start, end)) => {
                        caret = start;
                        mark = None;
                        text[start..end].iter().collect()
                    }
                    None => {
                        let start = line_start(&text, caret);
                        let mut end = line_end(&text, caret);
                        if end < text.len() {
                            end += 1;
                        }
                        text[start..end].iter().collect()
                    }
                };
                store_yank(yanked);
                LAST_PASTE.with(|cell| cell.set(None));
            }
            EditorOp::Cut => {
                if let Some((start, end)) = selection_range(mark, caret) {
                    store_yank(text[start..end].iter().collect());
                    if end > start {
                        text.drain(start..end);
                        caret = start;
                        changed = true;
                    }
                    mark = None;
                    LAST_PASTE.with(|cell| cell.set(None));
                }
            }
            EditorOp::Paste => {
                let (paste, ring_index) = paste_source();
                if !paste.is_empty() {
                    let inserted: Vec<char> = paste.chars().collect();
                    let count = inserted.len();
                    if let Some((start, end)) = selection_range(mark, caret) {
                        text.splice(start..end, inserted);
                        caret = start + count;
                        mark = None;
                        LAST_PASTE.with(|cell| cell.set(None));
                    } else if paste.ends_with('\n') {
                        let mut at = line_end(&text, caret);
                        if at < text.len() {
                            at += 1;
                        } else {
                            text.push('\n');
                            at = text.len();
                        }
                        text.splice(at..at, inserted);
                        caret = at;
                        LAST_PASTE.with(|cell| cell.set(None));
                    } else {
                        text.splice(caret..caret, inserted);
                        LAST_PASTE
                            .with(|cell| cell.set(ring_index.map(|index| (caret, count, index))));
                        caret += count;
                    }
                    changed = true;
                }
            }
            EditorOp::PastePop => {
                if let Some((start, length, index)) = LAST_PASTE.with(Cell::get)
                    && index > 0
                    && let Some(replacement) =
                        KILL_RING.with(|ring| ring.borrow().get(index - 1).cloned())
                {
                    let end = (start + length).min(text.len());
                    let inserted: Vec<char> = replacement.chars().collect();
                    let count = inserted.len();
                    text.splice(start..end, inserted);
                    caret = start + count;
                    LAST_PASTE.with(|cell| cell.set(Some((start, count, index - 1))));
                    changed = true;
                }
            }
        }
    }

    let value: String = text.iter().collect();
    if changed {
        textarea.set_value(&value);
    }
    let caret = caret.min(text.len());
    let mark = mark.map(|anchor| anchor.min(text.len()));
    MARK.with(|cell| cell.set(mark));
    // With a live mark the textarea shows the region; otherwise the caret sits
    // collapsed. This is what makes motions paint a visual-mode selection.
    match selection_range(mark, caret) {
        Some((lo, hi)) if hi > lo => {
            let _ = textarea.set_selection_range(lo as u32, hi as u32);
        }
        _ => {
            let _ = textarea.set_selection_range(caret as u32, caret as u32);
        }
    }

    if let Some(mode) = new_mode {
        state.editing.mode.set(mode);
    }

    if changed {
        state.set_buffer_text(kind, &id, value.clone());
    }
    changed
}

/// Moves the caret to a character on the current line, storing the find so `;`
/// and `,` can repeat it.
fn find_char(caret: &mut usize, text: &[char], target: &str, kind: FindKind) {
    if let Some(needle) = target.chars().next() {
        if let Some(index) = find_on_line(text, *caret, needle, kind) {
            *caret = index;
        }
        LAST_FIND.with(|cell| cell.set(Some((needle, kind))));
    }
}

/// The caret position an in-line find lands on, searching only the caret's line.
fn find_on_line(text: &[char], caret: usize, needle: char, kind: FindKind) -> Option<usize> {
    let lo = line_start(text, caret);
    let hi = line_end(text, caret);
    match kind {
        FindKind::Find => (caret + 1..hi).find(|index| text[*index] == needle),
        FindKind::Till => (caret + 1..hi)
            .find(|index| text[*index] == needle && *index > caret + 1)
            .map(|index| index - 1),
        FindKind::FindBack => (lo..caret).rev().find(|index| text[*index] == needle),
        FindKind::TillBack => (lo..caret)
            .rev()
            .find(|index| text[*index] == needle && index + 1 < caret)
            .map(|index| index + 1),
    }
}

/// The opposite direction of a find, for `,`.
fn opposite_find(kind: FindKind) -> FindKind {
    match kind {
        FindKind::Find => FindKind::FindBack,
        FindKind::FindBack => FindKind::Find,
        FindKind::Till => FindKind::TillBack,
        FindKind::TillBack => FindKind::Till,
    }
}

/// The index of the next match of `needle` from the caret, wrapping around the
/// buffer. Searches forward or backward; returns none when there is no match.
fn find_match(text: &[char], caret: usize, needle: &[char], forward: bool) -> Option<usize> {
    if needle.is_empty() || needle.len() > text.len() {
        return None;
    }
    let last = text.len() - needle.len();
    let matches = |index: usize| text[index..index + needle.len()] == *needle;
    if forward {
        let start = caret.min(last);
        (start + 1..=last)
            .find(|index| matches(*index))
            .or_else(|| (0..=start).find(|index| matches(*index)))
    } else {
        let start = caret.min(last);
        (0..start)
            .rev()
            .find(|index| matches(*index))
            .or_else(|| (start..=last).rev().find(|index| matches(*index)))
    }
}

/// Pushes yanked text onto the kill-ring, and into the pending named register
/// when one was set.
fn store_yank(yanked: String) {
    if yanked.is_empty() {
        return;
    }
    if let Some(register) = PENDING_REGISTER.with(Cell::take) {
        REGISTERS.with(|registers| {
            registers.borrow_mut().insert(register, yanked.clone());
        });
    }
    KILL_RING.with(|ring| {
        let mut ring = ring.borrow_mut();
        ring.push(yanked);
        while ring.len() > KILL_RING_LIMIT {
            ring.remove(0);
        }
    });
}

/// The text a paste inserts and its ring index (for yank-pop), reading the named
/// register when one is pending, otherwise the top of the kill-ring.
fn paste_source() -> (String, Option<usize>) {
    if let Some(register) = PENDING_REGISTER.with(Cell::take) {
        let text = REGISTERS.with(|registers| registers.borrow().get(&register).cloned());
        return (text.unwrap_or_default(), None);
    }
    KILL_RING.with(|ring| {
        let ring = ring.borrow();
        match ring.last() {
            Some(text) => (text.clone(), Some(ring.len() - 1)),
            None => (String::new(), None),
        }
    })
}

/// The ordered selection range `(start, end)` for an active mark, or none.
fn selection_range(mark: Option<usize>, caret: usize) -> Option<(usize, usize)> {
    mark.map(|anchor| {
        if anchor <= caret {
            (anchor, caret)
        } else {
            (caret, anchor)
        }
    })
}

/// The word boundaries around a caret.
fn word_bounds(text: &[char], caret: usize) -> (usize, usize) {
    let caret = caret.min(text.len());
    let mut start = caret;
    while start > 0 && is_word(text[start - 1]) {
        start -= 1;
    }
    let mut end = caret;
    while end < text.len() && is_word(text[end]) {
        end += 1;
    }
    (start, end)
}

/// The bounds of a text object around the caret: a word (`w`/`W`), or the text
/// inside (or, with `around`, including) the nearest bracket or quote pair the
/// spec names.
fn object_bounds(text: &[char], caret: usize, spec: &str, around: bool) -> Option<(usize, usize)> {
    match spec.chars().next()? {
        'w' | 'W' => {
            let (start, end) = word_bounds(text, caret);
            if start == end {
                return None;
            }
            if around {
                let mut stop = end;
                while stop < text.len() && (text[stop] == ' ' || text[stop] == '\t') {
                    stop += 1;
                }
                Some((start, stop))
            } else {
                Some((start, end))
            }
        }
        quote @ ('"' | '\'' | '`') => quote_bounds(text, caret, quote, around),
        '(' | ')' | 'b' => bracket_bounds(text, caret, '(', ')', around),
        '{' | '}' | 'B' => bracket_bounds(text, caret, '{', '}', around),
        '[' | ']' => bracket_bounds(text, caret, '[', ']', around),
        '<' | '>' => bracket_bounds(text, caret, '<', '>', around),
        _ => None,
    }
}

/// The bounds of the quote pair on the caret's line that encloses or follows it.
fn quote_bounds(text: &[char], caret: usize, quote: char, around: bool) -> Option<(usize, usize)> {
    let lo = line_start(text, caret);
    let hi = line_end(text, caret);
    let positions: Vec<usize> = (lo..hi).filter(|index| text[*index] == quote).collect();
    let pick = positions
        .chunks_exact(2)
        .find(|pair| pair[0] <= caret && caret <= pair[1])
        .or_else(|| positions.chunks_exact(2).find(|pair| pair[0] >= caret))?;
    let (open, close) = (pick[0], pick[1]);
    if around {
        Some((open, close + 1))
    } else {
        Some((open + 1, close))
    }
}

/// The bounds of the bracket pair enclosing the caret, matched with nesting.
fn bracket_bounds(
    text: &[char],
    caret: usize,
    open: char,
    close: char,
    around: bool,
) -> Option<(usize, usize)> {
    let mut depth = 0;
    let mut index = caret.min(text.len());
    let open_index = loop {
        if index == 0 {
            return None;
        }
        index -= 1;
        if text[index] == close {
            depth += 1;
        } else if text[index] == open {
            if depth == 0 {
                break index;
            }
            depth -= 1;
        }
    };
    let mut depth = 0;
    let mut index = caret;
    let close_index = loop {
        if index >= text.len() {
            return None;
        }
        if text[index] == open && index != open_index {
            depth += 1;
        } else if text[index] == close {
            if depth == 0 {
                break index;
            }
            depth -= 1;
        }
        index += 1;
    };
    if around {
        Some((open_index, close_index + 1))
    } else {
        Some((open_index + 1, close_index))
    }
}

/// The first-line start and last-line end of the block a selection touches.
fn block_bounds(text: &[char], start: usize, end: usize) -> (usize, usize) {
    let last = if end > start { end - 1 } else { end };
    (line_start(text, start), line_end(text, last))
}

/// Rewrites every line the selection touches through `map`, returning the block
/// start as the new caret. The substrate for region indent, outdent, and comment.
fn map_block_lines(
    text: &mut Vec<char>,
    start: usize,
    end: usize,
    map: impl Fn(&str) -> String,
) -> usize {
    let (lo, hi) = block_bounds(text, start, end);
    let block: String = text[lo..hi].iter().collect();
    let mapped = block.split('\n').map(&map).collect::<Vec<_>>().join("\n");
    text.splice(lo..hi, mapped.chars());
    lo
}

/// Removes up to four leading spaces (or one tab) from a line, the per-line work
/// of a region outdent.
fn outdent_line(line: &str) -> String {
    if let Some(rest) = line.strip_prefix('\t') {
        return rest.to_string();
    }
    let mut removed = 0;
    while removed < 4 && line[removed..].starts_with(' ') {
        removed += 1;
    }
    line[removed..].to_string()
}

/// Comments or uncomments every line of a block: uncomment when every non-blank
/// line already opens with the marker, otherwise comment.
fn comment_block(text: &mut Vec<char>, start: usize, end: usize, marker: &str) -> usize {
    let (lo, hi) = block_bounds(text, start, end);
    let block: String = text[lo..hi].iter().collect();
    let commented = block
        .split('\n')
        .filter(|line| !line.trim().is_empty())
        .all(|line| line.trim_start().starts_with(marker));
    map_block_lines(text, start, end, |line| {
        if line.trim().is_empty() {
            return line.to_string();
        }
        if commented {
            let indent: String = line
                .chars()
                .take_while(|character| *character == ' ' || *character == '\t')
                .collect();
            let rest = line.trim_start();
            let rest = rest.strip_prefix(marker).unwrap_or(rest);
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            format!("{indent}{rest}")
        } else {
            let indent: String = line
                .chars()
                .take_while(|character| *character == ' ' || *character == '\t')
                .collect();
            let rest = line.trim_start();
            format!("{indent}{marker} {rest}")
        }
    })
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
    if delta == 0 {
        return caret;
    }
    let column = caret - line_start(text, caret);
    let mut line_begin = line_start(text, caret);
    let steps = delta.unsigned_abs() as usize;
    if delta < 0 {
        for _ in 0..steps {
            if line_begin == 0 {
                break;
            }
            line_begin = line_start(text, line_begin - 1);
        }
    } else {
        for _ in 0..steps {
            let end = line_end(text, line_begin);
            if end >= text.len() {
                break;
            }
            line_begin = end + 1;
        }
    }
    (line_begin + column).min(line_end(text, line_begin))
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

/// The most compiled plugin ASTs kept. Each distinct source (every edit of an
/// editor plugin is a new one) caches an entry, so without a cap the map grows
/// for the whole session. When it fills, drop everything and recompile on
/// demand; the working set is a handful of enabled plugins, so the rebuild is
/// cheap and rare.
const CACHE_LIMIT: usize = 64;

fn compiled(source: &str) -> Option<AST> {
    let key = hash_source(source);
    CACHE.with(|cache| {
        if let Some(ast) = cache.borrow().get(&key) {
            return Some(ast.clone());
        }
        let ast = ENGINE.with(|engine| engine.compile(source).ok())?;
        let mut cache = cache.borrow_mut();
        if cache.len() >= CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, ast.clone());
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
    state.set_buffer_text(kind, &id, value.clone());
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
    state.editing.mode.set("normal".to_string());
    clear_mark();
}

/// Drops the region anchor, so a pointer click or buffer switch does not leave a
/// stale visual-mode selection that the next motion would extend.
pub fn clear_mark() {
    MARK.with(|cell| cell.set(None));
}
