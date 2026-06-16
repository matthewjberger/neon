//! A hand-written rhai scanner for the editor's highlight overlay. Pure Rust, no
//! grammar dependency: enough to color keywords, strings, numbers, comments, and
//! api command calls. Command recognition comes from the manifest the worker
//! sends, so it never drifts from what a script can actually call.

use std::collections::HashSet;

const KEYWORDS: &[&str] = &[
    "fn", "let", "const", "if", "else", "for", "in", "while", "loop", "return", "break", "continue",
    "switch", "import", "export", "global", "private", "true", "false", "throw", "try", "catch",
    "this",
];

/// Splits rhai source into (css class, text) runs for the highlight layer.
pub fn highlight(source: &str, commands: &HashSet<String>) -> Vec<(&'static str, String)> {
    let chars: Vec<char> = source.chars().collect();
    let count = chars.len();
    let mut runs: Vec<(&'static str, String)> = Vec::new();
    let mut index = 0;
    while index < count {
        let current = chars[index];
        if current == '/' && index + 1 < count && chars[index + 1] == '/' {
            let start = index;
            while index < count && chars[index] != '\n' {
                index += 1;
            }
            runs.push(("tok-comment", chars[start..index].iter().collect()));
        } else if current == '"' {
            let start = index;
            index += 1;
            while index < count {
                if chars[index] == '\\' && index + 1 < count {
                    index += 2;
                    continue;
                }
                let quote = chars[index] == '"';
                index += 1;
                if quote {
                    break;
                }
            }
            runs.push(("tok-string", chars[start..index].iter().collect()));
        } else if current.is_ascii_digit() {
            let start = index;
            while index < count && (chars[index].is_ascii_digit() || chars[index] == '.') {
                index += 1;
            }
            runs.push(("tok-number", chars[start..index].iter().collect()));
        } else if current.is_alphabetic() || current == '_' {
            let start = index;
            while index < count && (chars[index].is_alphanumeric() || chars[index] == '_') {
                index += 1;
            }
            let word: String = chars[start..index].iter().collect();
            let class = if KEYWORDS.contains(&word.as_str()) {
                "tok-keyword"
            } else if commands.contains(&word) {
                "tok-command"
            } else {
                "tok-plain"
            };
            runs.push((class, word));
        } else {
            let start = index;
            index += 1;
            while index < count {
                let next = chars[index];
                let token_start = (next == '/' && index + 1 < count && chars[index + 1] == '/')
                    || next == '"'
                    || next.is_ascii_digit()
                    || next.is_alphabetic()
                    || next == '_';
                if token_start {
                    break;
                }
                index += 1;
            }
            runs.push(("tok-plain", chars[start..index].iter().collect()));
        }
    }
    runs
}
