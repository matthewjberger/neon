//! The page side of the git bridge. Connects to the desktop git relay, asks for
//! a file's diff against HEAD when it opens or is saved, and stores the changed
//! lines so the editor gutter can mark them. A no-op in a plain browser.

use std::cell::RefCell;

use leptos::prelude::*;
use protocol::{GitClientMessage, GitServerMessage};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

use crate::state::EditorState;

const URL: &str = "ws://127.0.0.1:8795";
const RECONNECT_MS: i32 = 1000;

thread_local! {
    static SOCKET: RefCell<Option<WebSocket>> = const { RefCell::new(None) };
}

/// Opens the git relay and keeps it open, reconnecting on drop.
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
            && let Ok(message) = serde_json::from_str::<GitServerMessage>(&text)
        {
            match message {
                GitServerMessage::Diff { path, changes, .. } => {
                    state.git_changes.update(|map| {
                        map.insert(path, changes);
                    });
                }
                GitServerMessage::Status { branch, files, .. } => {
                    state.git_status.set((branch, files));
                }
                GitServerMessage::Done { .. } => {
                    refresh_status(state);
                    refresh_open_files(state);
                }
            }
        }
    });
    websocket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let onopen = Closure::<dyn FnMut()>::new(move || refresh_open_files(state));
    websocket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let onclose = Closure::<dyn FnMut()>::new(move || schedule_reconnect(state));
    websocket.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    SOCKET.with(|slot| *slot.borrow_mut() = Some(websocket));
}

fn schedule_reconnect(state: EditorState) {
    SOCKET.with(|slot| *slot.borrow_mut() = None);
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

/// Sends one request to the git relay, when the socket is open.
fn send(request: &GitClientMessage) {
    SOCKET.with(|slot| {
        if let Some(websocket) = slot.borrow().as_ref()
            && websocket.ready_state() == WebSocket::OPEN
            && let Ok(text) = serde_json::to_string(request)
        {
            let _ = websocket.send_with_str(&text);
        }
    });
}

/// Requests the diff for one file. Called when a file opens and when it is saved.
pub fn refresh(path: &str) {
    send(&GitClientMessage::DiffFile {
        request_id: 0,
        path: path.to_string(),
    });
}

/// Requests the repo status for the source-control panel.
pub fn refresh_status(state: EditorState) {
    let Some(root) = state.explorer.root.get_untracked() else {
        return;
    };
    send(&GitClientMessage::Status {
        request_id: 0,
        root,
    });
}

/// Stages a path, then the relay's `Done` reply refreshes the status.
pub fn stage(state: EditorState, path: &str) {
    if let Some(root) = state.explorer.root.get_untracked() {
        send(&GitClientMessage::Stage {
            request_id: 0,
            root,
            path: path.to_string(),
        });
    }
}

/// Unstages a path.
pub fn unstage(state: EditorState, path: &str) {
    if let Some(root) = state.explorer.root.get_untracked() {
        send(&GitClientMessage::Unstage {
            request_id: 0,
            root,
            path: path.to_string(),
        });
    }
}

/// Commits the staged changes with a message.
pub fn commit(state: EditorState, message: &str) {
    if let Some(root) = state.explorer.root.get_untracked() {
        send(&GitClientMessage::Commit {
            request_id: 0,
            root,
            message: message.to_string(),
        });
    }
}

/// Requests diffs for every open file, on connect.
fn refresh_open_files(state: EditorState) {
    let paths: Vec<String> = state
        .files
        .with_untracked(|files| files.iter().map(|file| file.path.clone()).collect());
    for path in paths {
        refresh(&path);
    }
}
