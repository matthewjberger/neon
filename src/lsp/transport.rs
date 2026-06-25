//! The wire layer. Open the websocket to the desktop relay, reconnect when it
//! drops, frame JSON-RPC requests and notifications, and translate file paths to
//! and from `file:` uris. Incoming messages are handed to `super::handle`.

use leptos::prelude::*;
use serde_json::{Value, json};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

use super::{client, handle};
use crate::state::EditorState;
use protocol::{LspClientMessage, LspServerMessage};

const LSP_URL: &str = "ws://127.0.0.1:8793";
const RECONNECT_MS: i32 = 1000;

pub(super) fn connect(state: EditorState) {
    let Ok(websocket) = WebSocket::new(LSP_URL) else {
        schedule_reconnect(state);
        return;
    };
    let open_state = state;
    let onopen = Closure::<dyn FnMut()>::new(move || {
        if let Some(root) = open_state.explorer.root.get_untracked() {
            send(&LspClientMessage::Start {
                root_uri: file_uri(&root),
            });
        }
    });
    websocket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        if let Some(text) = event.data().as_string()
            && let Ok(message) = serde_json::from_str::<LspServerMessage>(&text)
        {
            handle(state, message);
        }
    });
    websocket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let onclose = Closure::<dyn FnMut()>::new(move || {
        client(|client| client.ready = false);
        schedule_reconnect(state);
    });
    websocket.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    client(|client| client.socket = Some(websocket));
}

fn schedule_reconnect(state: EditorState) {
    client(|client| client.socket = None);
    if !state.lsp.started.get_untracked() {
        return;
    }
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

pub(super) fn send_request(method: &str, params: Value) {
    send_raw(json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }));
}

pub(super) fn send_request_id(id: i64, method: &str, params: Value) {
    send_raw(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
}

pub(super) fn notify(method: &str, params: Value) {
    send_raw(json!({ "jsonrpc": "2.0", "method": method, "params": params }));
}

pub(super) fn send_raw(message: Value) {
    send(&LspClientMessage::Rpc {
        json: message.to_string(),
    });
}

fn send(message: &LspClientMessage) {
    client(|client| {
        if let Some(websocket) = client.socket.as_ref()
            && websocket.ready_state() == WebSocket::OPEN
            && let Ok(text) = serde_json::to_string(message)
        {
            let _ = websocket.send_with_str(&text);
        }
    });
}

pub(super) fn file_uri(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    if normalized.starts_with('/') {
        format!("file://{normalized}")
    } else {
        format!("file:///{normalized}")
    }
}

pub(super) fn path_from_uri(uri: &str) -> String {
    let trimmed = uri
        .strip_prefix("file:///")
        .or_else(|| uri.strip_prefix("file://"))
        .unwrap_or(uri);
    trimmed.replace('/', "\\")
}
