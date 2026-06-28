//! The syntax-highlight bridge: a websocket relay that runs tree-sitter natively
//! and hands the page token spans. Tree-sitter and its grammars are C, so they
//! cannot ride in the page's wasm; instead the page sends a buffer's language and
//! source here and gets back `HighlightSpan`s, framed exactly like every other
//! desktop bridge. Languages without a grammar return no spans, so the page falls
//! back to its own scanner (which keeps the rhai command coloring).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use futures_util::{SinkExt, StreamExt};
use protocol::{HighlightClientMessage, HighlightServerMessage, HighlightSpan};
use tokio_tungstenite::tungstenite::Message;
use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};

const WS_ADDR: &str = "127.0.0.1:8794";

/// The token kinds the highlighter recognizes, in the order their index maps to
/// a CSS class in [`CLASSES`]. Listing both a base name and its dotted refinement
/// (e.g. `constant` and `constant.builtin`) lets tree-sitter pick the most
/// specific capture that a grammar's query emits.
const HIGHLIGHT_NAMES: &[&str] = &[
    "attribute",
    "comment",
    "constant",
    "constant.builtin",
    "constructor",
    "escape",
    "function",
    "function.builtin",
    "function.method",
    "keyword",
    "label",
    "module",
    "number",
    "operator",
    "property",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "string",
    "string.special",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
];

/// The CSS class for each entry in [`HIGHLIGHT_NAMES`], by the same index. The
/// classes reuse the editor's existing token palette and add a few new kinds,
/// all defined in `public/styles.css`.
const CLASSES: &[&str] = &[
    "tok-attribute",   // attribute
    "tok-comment",     // comment
    "tok-constant",    // constant
    "tok-constant",    // constant.builtin
    "tok-function",    // constructor
    "tok-string",      // escape
    "tok-function",    // function
    "tok-function",    // function.builtin
    "tok-function",    // function.method
    "tok-keyword",     // keyword
    "tok-keyword",     // label
    "tok-namespace",   // module
    "tok-number",      // number
    "tok-operator",    // operator
    "tok-property",    // property
    "tok-punctuation", // punctuation
    "tok-punctuation", // punctuation.bracket
    "tok-punctuation", // punctuation.delimiter
    "tok-string",      // string
    "tok-string",      // string.special
    "tok-keyword",     // tag
    "tok-type",        // type
    "tok-type",        // type.builtin
    "tok-variable",    // variable
    "tok-constant",    // variable.builtin
    "tok-variable",    // variable.parameter
];

/// The largest buffer the bridge parses. A file past this is left to the page's
/// own scanner rather than parsed and serialized span by span.
const MAX_SOURCE_BYTES: usize = 2_000_000;

static STARTED: AtomicBool = AtomicBool::new(false);

/// Starts the highlight relay on a background thread with its own runtime.
/// Idempotent.
pub fn start() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime.block_on(run_server()),
            Err(error) => log(&format!("failed to start the highlight runtime: {error}")),
        }
    });
}

async fn run_server() {
    let listener = match tokio::net::TcpListener::bind(WS_ADDR).await {
        Ok(listener) => listener,
        Err(error) => {
            log(&format!("failed to bind {WS_ADDR}: {error}"));
            return;
        }
    };
    log(&format!("highlight relay listening on ws://{WS_ADDR}"));
    loop {
        let Ok((stream, _addr)) = listener.accept().await else {
            continue;
        };
        tokio::spawn(async move {
            handle_page(stream).await;
        });
    }
}

async fn handle_page(stream: tokio::net::TcpStream) {
    let websocket = match tokio_tungstenite::accept_async(stream).await {
        Ok(websocket) => websocket,
        Err(error) => {
            log(&format!("highlight handshake failed: {error}"));
            return;
        }
    };
    let (mut sink, mut source) = websocket.split();
    while let Some(message) = source.next().await {
        let Ok(message) = message else {
            break;
        };
        let text = match message {
            Message::Text(text) => text,
            Message::Close(_) => break,
            _ => continue,
        };
        let Ok(request) = serde_json::from_str::<HighlightClientMessage>(&text) else {
            continue;
        };
        let response = handle(request).await;
        let Ok(json) = serde_json::to_string(&response) else {
            continue;
        };
        if sink.send(Message::Text(json)).await.is_err() {
            break;
        }
    }
}

async fn handle(request: HighlightClientMessage) -> HighlightServerMessage {
    let HighlightClientMessage::Highlight {
        request_id,
        language,
        text,
    } = request;
    if text.len() > MAX_SOURCE_BYTES {
        return HighlightServerMessage::Tokens {
            request_id,
            spans: Vec::new(),
        };
    }
    let spans = tokio::task::spawn_blocking(move || spans_for(&language, &text))
        .await
        .unwrap_or_default();
    HighlightServerMessage::Tokens { request_id, spans }
}

/// Computes the token spans for a buffer, or none when the language has no
/// grammar or the parse fails.
fn spans_for(language: &str, text: &str) -> Vec<HighlightSpan> {
    let Some(configuration) = configuration(language) else {
        return Vec::new();
    };
    let mut highlighter = Highlighter::new();
    let Ok(events) = highlighter.highlight(&configuration, text.as_bytes(), None, |_| None) else {
        return Vec::new();
    };
    let mut spans: Vec<HighlightSpan> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for event in events {
        match event {
            Ok(HighlightEvent::HighlightStart(Highlight(index))) => stack.push(index),
            Ok(HighlightEvent::HighlightEnd) => {
                stack.pop();
            }
            Ok(HighlightEvent::Source { start, end }) => {
                if start >= end {
                    continue;
                }
                let Some(index) = stack.last().copied() else {
                    continue;
                };
                let class = CLASSES[index];
                push_span(&mut spans, start as u32, end as u32, class);
            }
            Err(_) => break,
        }
    }
    spans
}

/// Appends a run, extending the previous one when it is the same class and
/// directly adjacent, so the page renders fewer DOM nodes.
fn push_span(spans: &mut Vec<HighlightSpan>, start: u32, end: u32, class: &str) {
    if let Some(last) = spans.last_mut()
        && last.end == start
        && last.class == class
    {
        last.end = end;
        return;
    }
    spans.push(HighlightSpan {
        start,
        end,
        class: class.to_string(),
    });
}

type Registry = Mutex<HashMap<&'static str, Option<Arc<HighlightConfiguration>>>>;

/// The configured tree-sitter highlighter for a language, built once and cached.
/// A language with no grammar caches `None`.
fn configuration(language: &str) -> Option<Arc<HighlightConfiguration>> {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    let key = canonical(language)?;
    let registry = REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    let mut registry = registry.lock().ok()?;
    registry
        .entry(key)
        .or_insert_with(|| build(key).map(Arc::new))
        .clone()
}

/// The grammar key a language maps to, or none when it has no grammar (rhai,
/// markdown, plaintext, and the rest stay on the page's own scanner).
fn canonical(language: &str) -> Option<&'static str> {
    match language {
        "rust" => Some("rust"),
        "json" => Some("json"),
        "javascript" | "typescript" => Some("javascript"),
        "css" => Some("css"),
        "toml" => Some("toml"),
        _ => None,
    }
}

/// Builds and configures the highlighter for a grammar key.
fn build(key: &'static str) -> Option<HighlightConfiguration> {
    let names: Vec<String> = HIGHLIGHT_NAMES
        .iter()
        .map(|name| name.to_string())
        .collect();
    let mut configuration = match key {
        "rust" => HighlightConfiguration::new(
            tree_sitter_rust::LANGUAGE.into(),
            "rust",
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            tree_sitter_rust::INJECTIONS_QUERY,
            "",
        ),
        "json" => HighlightConfiguration::new(
            tree_sitter_json::LANGUAGE.into(),
            "json",
            tree_sitter_json::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        "javascript" => HighlightConfiguration::new(
            tree_sitter_javascript::LANGUAGE.into(),
            "javascript",
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            tree_sitter_javascript::INJECTIONS_QUERY,
            tree_sitter_javascript::LOCALS_QUERY,
        ),
        "css" => HighlightConfiguration::new(
            tree_sitter_css::LANGUAGE.into(),
            "css",
            tree_sitter_css::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        "toml" => HighlightConfiguration::new(
            tree_sitter_toml_ng::LANGUAGE.into(),
            "toml",
            tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        _ => return None,
    }
    .ok()?;
    configuration.configure(&names);
    Some(configuration)
}

fn log(message: &str) {
    eprintln!("[highlight] {message}");
}
