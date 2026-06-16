//! Synchronous plugin source check for the agent path. Mirrors the language
//! worker (`lang/`): a rhai compile for syntax errors plus a scan for
//! `commands.x(` calls that are not a known command or helper. The language
//! worker drives the editor's live diagnostics off-thread; this answers the
//! agent in one round trip so it knows whether its edit compiled.

use std::collections::HashSet;

use leptos::prelude::*;
use protocol::{Diagnostic, Severity};
use rhai::Engine;

use crate::state::EditorState;

const BUILTINS: &[&str] = &[
    "push",
    "tag",
    "last",
    "log",
    "print",
    "len",
    "clear",
    "pad",
    "to_float",
    "to_int",
    "abs",
    "sin",
    "cos",
    "tan",
    "sqrt",
    "floor",
    "ceil",
    "round",
    "min",
    "max",
    "random",
    "random_range",
    "random_int",
    "entity_ref",
    "result",
];

/// Compile-check a plugin's source and flag unknown command calls.
pub fn check(state: EditorState, source: &str) -> Vec<Diagnostic> {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(0, 0);
    engine.set_max_operations(0);
    if let Err(error) = engine.compile(source) {
        let position = error.position();
        return vec![Diagnostic {
            message: error.to_string(),
            line: position.line().unwrap_or(0) as u32,
            column: position.position().unwrap_or(0) as u32,
            severity: Severity::Error,
        }];
    }
    unknown_calls(state, source)
}

fn vocabulary(state: EditorState) -> HashSet<String> {
    let mut set = HashSet::new();
    state.commands.with_untracked(|commands| {
        for command in commands {
            set.insert(command.method.clone());
        }
    });
    state.stdlib.with_untracked(|modules| {
        for module in modules {
            for helper in &module.helpers {
                set.insert(helper.name.clone());
            }
        }
    });
    for builtin in BUILTINS {
        set.insert((*builtin).to_string());
    }
    set
}

fn unknown_calls(state: EditorState, source: &str) -> Vec<Diagnostic> {
    let vocabulary = vocabulary(state);
    if vocabulary.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let needle = b"commands.";
    let mut index = 0;
    while index + needle.len() <= bytes.len() {
        if &bytes[index..index + needle.len()] == needle {
            let start = index + needle.len();
            let mut end = start;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            let mut after = end;
            while after < bytes.len() && bytes[after] == b' ' {
                after += 1;
            }
            if end > start && after < bytes.len() && bytes[after] == b'(' {
                let name = &source[start..end];
                if !vocabulary.contains(name) {
                    let (line, column) = line_col(source, start);
                    out.push(Diagnostic {
                        message: format!("unknown command or helper: commands.{name}"),
                        line,
                        column,
                        severity: Severity::Warning,
                    });
                }
            }
            index = end;
        } else {
            index += 1;
        }
    }
    out
}

fn line_col(source: &str, index: usize) -> (u32, u32) {
    let mut line = 1_u32;
    let mut column = 1_u32;
    for (offset, character) in source.char_indices() {
        if offset >= index {
            break;
        }
        if character == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}
