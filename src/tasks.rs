//! The page side of the task runner. Connects to the desktop relay, runs a
//! process in the workspace, and streams its output into the task panel. Drives
//! the Rust dev loop: cargo check, build, test, and run.

use std::cell::{Cell, RefCell};

use leptos::prelude::*;
use protocol::{TaskRequest, TaskResponse};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

use crate::state::EditorState;

const URL: &str = "ws://127.0.0.1:8794";
const RECONNECT_MS: i32 = 1000;
const OUTPUT_LIMIT: usize = 4000;

thread_local! {
    static SOCKET: RefCell<Option<WebSocket>> = const { RefCell::new(None) };
    static NEXT_ID: Cell<u64> = const { Cell::new(1) };
    static CURRENT: Cell<u64> = const { Cell::new(0) };
}

/// Opens the task relay and keeps it open, reconnecting on drop.
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
            && let Ok(response) = serde_json::from_str::<TaskResponse>(&text)
        {
            dispatch(state, response);
        }
    });
    websocket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();
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

fn send(request: &TaskRequest) {
    SOCKET.with(|slot| {
        if let Some(websocket) = slot.borrow().as_ref()
            && websocket.ready_state() == WebSocket::OPEN
            && let Ok(text) = serde_json::to_string(request)
        {
            let _ = websocket.send_with_str(&text);
        }
    });
}

/// Runs a program in the workspace root, streaming its output to the panel.
pub fn run(state: EditorState, program: &str, args: &[&str]) {
    let Some(cwd) = state.workspace_root.get_untracked() else {
        state.status.set("Open a folder first".to_string());
        return;
    };
    let id = NEXT_ID.with(|next| {
        let value = next.get();
        next.set(value + 1);
        value
    });
    CURRENT.with(|current| current.set(id));
    state
        .task_output
        .set(vec![format!("$ {program} {}", args.join(" "))]);
    state.task_running.set(true);
    state.task_open.set(true);
    send(&TaskRequest::Run {
        id,
        program: program.to_string(),
        args: args.iter().map(|arg| arg.to_string()).collect(),
        cwd,
    });
}

/// Cancels the running task.
pub fn cancel(state: EditorState) {
    let id = CURRENT.with(Cell::get);
    if id != 0 {
        send(&TaskRequest::Cancel { id });
        state.task_running.set(false);
    }
}

fn dispatch(state: EditorState, response: TaskResponse) {
    let current = CURRENT.with(Cell::get);
    match response {
        TaskResponse::Started { .. } => {}
        TaskResponse::Line { id, text } if id == current => push(state, text),
        TaskResponse::Exited { id, code } if id == current => {
            push(
                state,
                match code {
                    Some(0) => "exited successfully".to_string(),
                    Some(code) => format!("exited with code {code}"),
                    None => "stopped".to_string(),
                },
            );
            state.task_running.set(false);
        }
        TaskResponse::Error { id, message } if id == current => {
            push(state, message);
            state.task_running.set(false);
        }
        _ => {}
    }
}

fn push(state: EditorState, line: String) {
    state.task_output.update(|output| {
        output.push(line);
        let overflow = output.len().saturating_sub(OUTPUT_LIMIT);
        if overflow > 0 {
            output.drain(0..overflow);
        }
    });
}
