//! The page side of the terminal. Connects to the desktop PTY relay, opens a
//! shell, holds the emulator's screen grid for the renderer, and sends encoded
//! keystrokes and resizes back.

use std::cell::RefCell;

use leptos::prelude::*;
use protocol::{TerminalClientMessage, TerminalServerMessage};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

use crate::state::EditorState;

const URL: &str = "ws://127.0.0.1:8794";
const RECONNECT_MS: i32 = 1000;

thread_local! {
    static SOCKET: RefCell<Option<WebSocket>> = const { RefCell::new(None) };
}

/// Opens the terminal relay and keeps it open, reconnecting on drop.
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
            && let Ok(message) = serde_json::from_str::<TerminalServerMessage>(&text)
        {
            dispatch(state, message);
        }
    });
    websocket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();
    let onopen = Closure::<dyn FnMut()>::new(move || state.term_connected.set(true));
    websocket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();
    let onclose = Closure::<dyn FnMut()>::new(move || {
        state.term_connected.set(false);
        schedule_reconnect(state);
    });
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

fn send(message: &TerminalClientMessage) {
    SOCKET.with(|slot| {
        if let Some(websocket) = slot.borrow().as_ref()
            && websocket.ready_state() == WebSocket::OPEN
            && let Ok(text) = serde_json::to_string(message)
        {
            let _ = websocket.send_with_str(&text);
        }
    });
}

/// Opens the PTY for the workspace at the given grid size.
pub fn open(state: EditorState, cols: u16, rows: u16) {
    let cwd = state.workspace_root.get_untracked().unwrap_or_default();
    send(&TerminalClientMessage::Open { cols, rows, cwd });
}

/// Resizes the PTY and emulator.
pub fn resize(cols: u16, rows: u16) {
    send(&TerminalClientMessage::Resize { cols, rows });
}

/// Sends raw bytes (encoded keystrokes) to the PTY.
pub fn send_input(bytes: Vec<u8>) {
    send(&TerminalClientMessage::Input { bytes });
}

/// Sends Ctrl+C to the shell.
pub fn interrupt() {
    send_input(vec![3]);
}

/// Runs a command line in the terminal, opening it if needed and queuing the
/// command until the shell is ready.
pub fn run(state: EditorState, command: &str) {
    let line = format!("{command}\r");
    state.terminal_open.set(true);
    if state.term_grid.get_untracked().is_some() {
        send_input(line.into_bytes());
    } else {
        state.term_pending.set(Some(line));
    }
}

fn dispatch(state: EditorState, message: TerminalServerMessage) {
    match message {
        TerminalServerMessage::Grid(grid) => {
            state.term_grid.set(Some(grid));
            if let Some(pending) = state.term_pending.get_untracked() {
                state.term_pending.set(None);
                send_input(pending.into_bytes());
            }
        }
        TerminalServerMessage::Exited => {
            state.term_grid.set(None);
        }
    }
}
