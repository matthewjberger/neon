//! The page side of the engine-worker conversation. Data only; behavior is the
//! free functions below. Maps each `WorkerMessage` to a signal write and sends
//! `ClientMessage`s as `postMessage` envelopes.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use protocol::{CANVAS_KEY, ClientMessage, LogKind, MESSAGE_KEY, WorkerMessage};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{MessageEvent, OffscreenCanvas, Worker, WorkerOptions, WorkerType};

use crate::state::EditorState;

const LOG_LIMIT: usize = 300;

#[derive(Clone)]
pub struct Bridge {
    worker: Worker,
}

/// Spawns the engine worker, wires its messages to the state signals, and sends
/// `Init` with the transferred canvas.
pub fn connect(offscreen: OffscreenCanvas, width: f32, height: f32, state: EditorState) -> Bridge {
    let options = WorkerOptions::new();
    options.set_type(WorkerType::Module);
    let worker =
        Worker::new_with_options("runtime/worker.js", &options).expect("failed to spawn worker");

    let relay_socket: crate::relay::RelaySocket = Rc::new(RefCell::new(None));
    let response_socket = relay_socket.clone();

    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let data = event.data();
        let Ok(payload) = js_sys::Reflect::get(&data, &JsValue::from_str(MESSAGE_KEY)) else {
            return;
        };
        let Ok(message) = serde_wasm_bindgen::from_value::<WorkerMessage>(payload) else {
            return;
        };
        match message {
            WorkerMessage::Ready {
                adapter,
                commands,
                stdlib,
                ..
            } => {
                state.adapter.set(adapter);
                state.commands.set(commands);
                state.stdlib.set(stdlib);
                state.ready.set(true);
            }
            WorkerMessage::Stats { fps, entity_count } => {
                state.fps.set(fps);
                state.entity_count.set(entity_count);
            }
            WorkerMessage::Busy { active } => state.busy.set(active),
            WorkerMessage::Selected { detail } => state.selected.set(detail),
            WorkerMessage::Report { entries } => {
                state.log.update(|log| {
                    log.extend(entries);
                    if log.len() > LOG_LIMIT {
                        let excess = log.len() - LOG_LIMIT;
                        log.drain(0..excess);
                    }
                });
            }
            WorkerMessage::PluginError { message, .. } => {
                state.log.update(|log| {
                    log.push(protocol::LogEntry {
                        kind: LogKind::Error,
                        label: "error".to_string(),
                        detail: message,
                    });
                });
            }
            WorkerMessage::Agent(response) => {
                crate::relay::send_response(&response_socket, &response);
            }
        }
    });
    worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let bridge = Bridge { worker };
    crate::relay::start(state, bridge.clone(), relay_socket);
    send_init(&bridge, offscreen, width, height);
    bridge
}

/// Forwards a message to the worker inside the `{ message }` envelope.
pub fn send(bridge: &Bridge, message: &ClientMessage) {
    let envelope = js_sys::Object::new();
    let value = serde_wasm_bindgen::to_value(message).unwrap_or(JsValue::NULL);
    let _ = js_sys::Reflect::set(&envelope, &JsValue::from_str(MESSAGE_KEY), &value);
    let _ = bridge.worker.post_message(&envelope);
}

/// Pushes the whole plugin set to the worker, which rebuilds and reruns it.
pub fn sync_plugins(bridge: &Bridge, state: EditorState) {
    let plugins = state.plugins.get_untracked();
    send(bridge, &ClientMessage::SetPlugins { plugins });
}

fn send_init(bridge: &Bridge, canvas: OffscreenCanvas, width: f32, height: f32) {
    let envelope = js_sys::Object::new();
    let value = serde_wasm_bindgen::to_value(&ClientMessage::Init { width, height })
        .unwrap_or(JsValue::NULL);
    let _ = js_sys::Reflect::set(&envelope, &JsValue::from_str(MESSAGE_KEY), &value);
    let _ = js_sys::Reflect::set(&envelope, &JsValue::from_str(CANVAS_KEY), &canvas);
    let transfer = js_sys::Array::of1(&canvas);
    let _ = bridge
        .worker
        .post_message_with_transfer(&envelope, &transfer);
}
