//! Completion for rhai buffers. rust-analyzer covers Rust files; this fills the
//! plugin buffers (scene, editor, built-in) with rhai keywords, the scene
//! command vocabulary, the standard-library helpers, and the editor op names,
//! filtered by the word at the caret. Acceptance reuses the LSP popup path.

use leptos::prelude::*;

use crate::state::{CompletionEntry, CompletionMenu, EditorState, PluginKind};

const KEYWORDS: &[&str] = &[
    "fn", "let", "const", "if", "else", "switch", "for", "in", "while", "loop", "break",
    "continue", "return", "true", "false", "throw", "try", "catch", "import", "export", "as",
    "global", "print", "debug", "type_of", "range",
];

const EDITOR_OPS: &[&str] = &[
    "Consume",
    "SetMode",
    "SetStatus",
    "Insert",
    "Move",
    "MoveLine",
    "LineStart",
    "LineEnd",
    "NextWord",
    "PrevWord",
    "DeleteForward",
    "DeleteBackward",
    "DeleteLine",
    "DeleteToLineEnd",
    "DeleteWordForward",
    "DeleteWordBackward",
    "DuplicateLine",
    "MoveLineUp",
    "MoveLineDown",
    "JoinLines",
    "Indent",
    "Outdent",
    "SmartLineStart",
    "UpperCaseWord",
    "LowerCaseWord",
    "SortLines",
    "DeleteTrailingWhitespace",
    "ToggleComment",
    "FindChar",
    "RunCommand",
    "OpenPalette",
    "ShowMenu",
    "HideMenu",
];

/// Offers rhai completion in the focused plugin buffer, anchored at the caret.
pub fn rhai_complete(state: EditorState) {
    let buffer = state.focused_buffer();
    let editor = match buffer.kind {
        PluginKind::Editor => true,
        PluginKind::Scene | PluginKind::Builtin => false,
        PluginKind::File => return,
    };
    let Some(element) = crate::components::find::active() else {
        return;
    };
    let value = element.value();
    let caret = element.selection_start().ok().flatten().unwrap_or(0);
    let prefix = word_prefix(&value, caret);
    if prefix.len() < 2 {
        state.completion.set(None);
        return;
    }
    let needle = prefix.to_lowercase();

    let mut entries: Vec<CompletionEntry> = Vec::new();
    let mut push = |label: &str, kind: &str| {
        if label.to_lowercase().starts_with(&needle) && label != prefix {
            entries.push(CompletionEntry {
                label: label.to_string(),
                insert: label.to_string(),
                detail: String::new(),
                kind: kind.to_string(),
            });
        }
    };
    for keyword in KEYWORDS {
        push(keyword, "kw");
    }
    if editor {
        for op in EDITOR_OPS {
            push(op, "op");
        }
    } else {
        for command in state.commands.get_untracked() {
            push(&command.variant, "cmd");
        }
        for module in state.stdlib.get_untracked() {
            for helper in &module.helpers {
                push(&helper.name, "fn");
            }
        }
    }
    entries.truncate(40);
    if entries.is_empty() {
        state.completion.set(None);
        return;
    }

    let (line, column) = line_column(&value, caret);
    let (x, top) = crate::caret::cell(&element, line, column);
    let y = top + crate::caret::line_height(&element);
    state.completion.set(Some(CompletionMenu {
        items: entries,
        x,
        y,
        prefix,
    }));
    state.completion_index.set(0);
}

fn word_prefix(value: &str, caret: u32) -> String {
    let mut seen = 0;
    let mut word = String::new();
    for character in value.chars() {
        if seen >= caret {
            break;
        }
        if character.is_alphanumeric() || character == '_' {
            word.push(character);
        } else {
            word.clear();
        }
        seen += character.len_utf16() as u32;
    }
    word
}

fn line_column(value: &str, caret: u32) -> (u32, u32) {
    let mut line = 0;
    let mut column = 0;
    let mut seen = 0;
    for character in value.chars() {
        if seen >= caret {
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
