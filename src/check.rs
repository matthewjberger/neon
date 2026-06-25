//! Synchronous plugin source check for the agent path. Mirrors the language
//! worker (`lang/`): a rhai compile for syntax errors plus a scan for
//! `commands.x(` calls that are not a known command or helper. The language
//! worker drives the editor's live diagnostics off-thread; this answers the
//! agent in one round trip so it knows whether its edit compiled.

use std::collections::HashSet;

use leptos::prelude::*;
use protocol::{Diagnostic, RHAI_BUILTINS, Severity, unknown_command_calls};
use rhai::Engine;

use crate::state::EditorState;

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
    unknown_command_calls(source, &vocabulary(state))
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
    for builtin in RHAI_BUILTINS {
        set.insert((*builtin).to_string());
    }
    set
}
