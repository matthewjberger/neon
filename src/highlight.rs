//! A hand-written multi-language scanner for the editor's highlight overlay.
//! Pure Rust, no grammar dependency: enough to color keywords, strings, numbers,
//! and comments per language, plus api command calls for rhai. Command
//! recognition comes from the manifest the worker sends, so it never drifts from
//! what a script can actually call.

use std::collections::HashSet;

const RHAI_KEYWORDS: &[&str] = &[
    "fn", "let", "const", "if", "else", "for", "in", "while", "loop", "return", "break",
    "continue", "switch", "import", "export", "global", "private", "true", "false", "throw", "try",
    "catch", "this",
];

const RUST_KEYWORDS: &[&str] = &[
    "fn", "let", "const", "static", "mut", "if", "else", "match", "for", "while", "loop", "in",
    "return", "break", "continue", "struct", "enum", "trait", "impl", "pub", "use", "mod", "self",
    "Self", "super", "crate", "as", "where", "ref", "move", "dyn", "async", "await", "unsafe",
    "extern", "type", "true", "false", "Some", "None", "Ok", "Err", "Box", "Vec", "String",
    "Option", "Result",
];

const TOML_KEYWORDS: &[&str] = &["true", "false"];
const JSON_KEYWORDS: &[&str] = &["true", "false", "null"];

const JS_KEYWORDS: &[&str] = &[
    "function",
    "let",
    "const",
    "var",
    "if",
    "else",
    "for",
    "while",
    "return",
    "break",
    "continue",
    "class",
    "new",
    "import",
    "export",
    "from",
    "default",
    "true",
    "false",
    "null",
    "undefined",
    "async",
    "await",
    "this",
];

fn keywords(language: &str) -> &'static [&'static str] {
    match language {
        "rust" => RUST_KEYWORDS,
        "toml" => TOML_KEYWORDS,
        "json" => JSON_KEYWORDS,
        "javascript" | "typescript" => JS_KEYWORDS,
        _ => RHAI_KEYWORDS,
    }
}

fn line_comment(language: &str) -> Option<&'static str> {
    match language {
        "toml" => Some("#"),
        "json" | "markdown" | "plaintext" => None,
        _ => Some("//"),
    }
}

fn block_comments(language: &str) -> bool {
    matches!(
        language,
        "rust" | "javascript" | "typescript" | "wgsl" | "css"
    )
}

/// Splits source into (css class, text) runs for the highlight layer.
pub fn highlight(
    source: &str,
    language: &str,
    commands: &HashSet<String>,
) -> Vec<(&'static str, String)> {
    let chars: Vec<char> = source.chars().collect();
    let count = chars.len();
    let keyword_set = keywords(language);
    let line = line_comment(language).and_then(|prefix| prefix.chars().next());
    let blocks = block_comments(language);
    let highlight_commands = matches!(language, "rhai");
    let mut runs: Vec<(&'static str, String)> = Vec::new();
    let mut index = 0;
    while index < count {
        let current = chars[index];
        if blocks && current == '/' && index + 1 < count && chars[index + 1] == '*' {
            let start = index;
            index += 2;
            while index < count
                && !(chars[index] == '*' && index + 1 < count && chars[index + 1] == '/')
            {
                index += 1;
            }
            index = (index + 2).min(count);
            runs.push(("tok-comment", chars[start..index].iter().collect()));
        } else if line == Some(current)
            && (current != '/' || (index + 1 < count && chars[index + 1] == '/'))
        {
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
            while index < count && (chars[index].is_ascii_alphanumeric() || chars[index] == '.') {
                index += 1;
            }
            runs.push(("tok-number", chars[start..index].iter().collect()));
        } else if current.is_alphabetic() || current == '_' {
            let start = index;
            while index < count && (chars[index].is_alphanumeric() || chars[index] == '_') {
                index += 1;
            }
            let word: String = chars[start..index].iter().collect();
            let class = if keyword_set.contains(&word.as_str()) {
                "tok-keyword"
            } else if highlight_commands && commands.contains(&word) {
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
                let token_start =
                    (blocks && next == '/' && index + 1 < count && chars[index + 1] == '*')
                        || (line == Some(next)
                            && (next != '/' || (index + 1 < count && chars[index + 1] == '/')))
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
