//! The page side of the native tree-sitter bridge. The grammars are C and run in
//! the desktop shell, so the page sends a buffer's language and source over a
//! websocket and caches the token spans the shell parses back. The editor overlay
//! reads those spans through [`runs_for`]; until they arrive (or with no shell, in
//! a plain browser) it falls back to the built-in scanner in `highlight.rs`.
//!
//! Only the languages with a single clean grammar go through the bridge (rust,
//! json, javascript, typescript, css, toml). Everything else, rhai included,
//! stays on the built-in scanner, which keeps the manifest-driven command
//! coloring tree-sitter has no notion of.

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use leptos::prelude::*;
use protocol::{HighlightClientMessage, HighlightServerMessage, HighlightSpan};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

use crate::state::EditorState;

const URL: &str = "ws://127.0.0.1:8794";
const RECONNECT_MS: i32 = 1000;
/// The most parsed buffers kept at once. Each distinct (language, text) caches an
/// entry; when the map fills it is cleared, since the working set is the few open
/// buffers and a rebuild is one round trip.
const CACHE_LIMIT: usize = 24;

/// A buffer's parsed spans, kept with the exact text they describe so a stale
/// cache entry is never painted over edited text.
struct Cached {
    text: String,
    spans: Vec<HighlightSpan>,
}

struct Client {
    socket: Option<WebSocket>,
    next_id: u32,
    /// In-flight requests: the request id to the cache key and the text it was
    /// sent for.
    pending: HashMap<u32, (u64, String)>,
    /// Parsed spans by `key(language, text)`.
    cache: HashMap<u64, Cached>,
}

impl Client {
    fn new() -> Self {
        Self {
            socket: None,
            next_id: 0,
            pending: HashMap::new(),
            cache: HashMap::new(),
        }
    }
}

thread_local! {
    static CLIENT: RefCell<Client> = RefCell::new(Client::new());
}

fn client<R>(action: impl FnOnce(&mut Client) -> R) -> R {
    CLIENT.with(|client| action(&mut client.borrow_mut()))
}

/// Whether a language is parsed through the bridge. Everything else falls back to
/// the built-in scanner.
pub fn supported(language: &str) -> bool {
    matches!(
        language,
        "rust" | "json" | "javascript" | "typescript" | "css" | "toml"
    )
}

fn key(language: &str, text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    language.hash(&mut hasher);
    text.hash(&mut hasher);
    hasher.finish()
}

/// Opens the highlight relay and keeps it open, reconnecting on drop.
pub fn start(state: EditorState) {
    connect(state);
}

fn connect(state: EditorState) {
    let Ok(websocket) = WebSocket::new(URL) else {
        schedule_reconnect(state);
        return;
    };
    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        if let Some(text) = event.data().as_string()
            && let Ok(message) = serde_json::from_str::<HighlightServerMessage>(&text)
        {
            receive(state, message);
        }
    });
    websocket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // Nudge the repaint tick on connect so the request effect re-fires and the
    // already-open buffers get parsed, not just the ones edited after the socket
    // came up.
    let onopen = Closure::<dyn FnMut()>::new(move || {
        state
            .editing
            .highlight
            .update(|tick| *tick = tick.wrapping_add(1));
    });
    websocket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let onclose = Closure::<dyn FnMut()>::new(move || schedule_reconnect(state));
    websocket.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    client(|client| client.socket = Some(websocket));
}

fn schedule_reconnect(state: EditorState) {
    client(|client| {
        client.socket = None;
        client.pending.clear();
    });
    let Some(window) = web_sys::window() else {
        return;
    };
    let callback = Closure::<dyn FnMut()>::new(move || connect(state));
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        RECONNECT_MS,
    );
    callback.forget();
}

fn receive(state: EditorState, message: HighlightServerMessage) {
    let HighlightServerMessage::Tokens { request_id, spans } = message;
    let resolved = client(|client| {
        let Some((cache_key, text)) = client.pending.remove(&request_id) else {
            return false;
        };
        if client.cache.len() >= CACHE_LIMIT {
            client.cache.clear();
        }
        client.cache.insert(cache_key, Cached { text, spans });
        true
    });
    if resolved {
        // Repaint the overlay so the freshly parsed spans replace the fallback.
        state
            .editing
            .highlight
            .update(|tick| *tick = tick.wrapping_add(1));
    }
}

/// Asks the shell to parse a buffer, unless its spans are already cached or a
/// request for the same text is already in flight. A no-op for an unsupported
/// language or with no shell connected.
pub fn request(language: &str, text: String) {
    if !supported(language) {
        return;
    }
    let cache_key = key(language, &text);
    let send = client(|client| {
        if client.cache.contains_key(&cache_key) {
            return None;
        }
        if client
            .pending
            .values()
            .any(|(pending_key, _)| *pending_key == cache_key)
        {
            return None;
        }
        let socket = client.socket.as_ref()?;
        if socket.ready_state() != WebSocket::OPEN {
            return None;
        }
        let request_id = client.next_id.wrapping_add(1);
        client.next_id = request_id;
        client.pending.insert(request_id, (cache_key, text.clone()));
        Some(request_id)
    });
    let Some(request_id) = send else {
        return;
    };
    let message = HighlightClientMessage::Highlight {
        request_id,
        language: language.to_string(),
        text,
    };
    if let Ok(json) = serde_json::to_string(&message) {
        client(|client| {
            if let Some(socket) = client.socket.as_ref() {
                let _ = socket.send_with_str(&json);
            }
        });
    }
}

/// The token runs (CSS class, text) tiling the window `[window_start, window_end)`
/// of `text`, from cached tree-sitter spans. `None` when the language is
/// unsupported or no fresh spans are cached, so the caller paints with the
/// built-in scanner instead. Offsets are UTF-8 byte positions into `text`.
pub fn runs_for(
    language: &str,
    text: &str,
    window_start: usize,
    window_end: usize,
) -> Option<Vec<(String, String)>> {
    if !supported(language) || window_end > text.len() || window_start > window_end {
        return None;
    }
    let cache_key = key(language, text);
    client(|client| {
        let cached = client.cache.get(&cache_key)?;
        if cached.text != text {
            return None;
        }
        Some(build_runs(text, &cached.spans, window_start, window_end))
    })
}

/// Tiles the window with the spans that intersect it, filling every gap with a
/// plain run so the runs reproduce the window text exactly, byte for byte.
fn build_runs(
    text: &str,
    spans: &[HighlightSpan],
    window_start: usize,
    window_end: usize,
) -> Vec<(String, String)> {
    let mut runs: Vec<(String, String)> = Vec::new();
    let mut position = window_start;
    for span in spans {
        let span_end = span.end as usize;
        if span_end <= position {
            continue;
        }
        if span.start as usize >= window_end {
            break;
        }
        // Never move backwards: clamp the span to what is still untiled. A failed
        // slice leaves `position` so the next gap reabsorbs the range, keeping the
        // runs a contiguous, non-overlapping tiling of the window.
        let start = (span.start as usize).max(position);
        let end = span_end.min(window_end);
        if start > position {
            if let Some(plain) = text.get(position..start) {
                runs.push(("tok-plain".to_string(), plain.to_string()));
            }
            position = start;
        }
        if end > start
            && let Some(piece) = text.get(start..end)
        {
            runs.push((span.class.clone(), piece.to_string()));
            position = end;
        }
    }
    if position < window_end
        && let Some(plain) = text.get(position..window_end)
    {
        runs.push(("tok-plain".to_string(), plain.to_string()));
    }
    runs
}
