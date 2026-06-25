//! The shared rhai plugin lint. Both the page (`neon`, synchronous, for the
//! agent round trip) and the language worker (`lang`, off-thread, for the live
//! diagnostics strip) flag `commands.x(` calls that are not a known command or
//! standard-library helper. The scan lives here so the two callers cannot drift.

use std::collections::HashSet;

use crate::{Diagnostic, Severity};

/// Rhai globals and number helpers that are always in scope, so a call on one is
/// never flagged as unknown.
pub const RHAI_BUILTINS: &[&str] = &[
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

/// Flags every `commands.name(` call whose `name` is not in `known`, as a
/// warning anchored at the call. An empty vocabulary yields nothing, so a source
/// checked before the worker reports its command set is never falsely flagged.
pub fn unknown_command_calls(source: &str, known: &HashSet<String>) -> Vec<Diagnostic> {
    if known.is_empty() {
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
                if !known.contains(name) {
                    let (line, column) = line_column(source, start);
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

fn line_column(source: &str, index: usize) -> (u32, u32) {
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
