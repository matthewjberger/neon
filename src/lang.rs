//! The page side of the language-worker conversation. The language worker links
//! only `rhai`: it compile-checks plugin source and validates command calls off
//! the render thread, so typing never competes with the engine. Diagnostics come
//! back keyed by request id.

use leptos::prelude::*;
use protocol::{CommandInfo, LangRequest, LangResponse, MESSAGE_KEY, StdModule};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

use crate::state::EditorState;

#[derive(Clone)]
pub struct Lang {
    worker: Worker,
}

/// Spawns the language worker and routes its diagnostics to the state.
pub fn connect(state: EditorState) -> Lang {
    let options = WorkerOptions::new();
    options.set_type(WorkerType::Module);
    let worker = Worker::new_with_options("runtime/lang_worker.js", &options)
        .expect("failed to spawn language worker");

    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let data = event.data();
        let Ok(payload) = js_sys::Reflect::get(&data, &JsValue::from_str(MESSAGE_KEY)) else {
            return;
        };
        let Ok(message) = serde_wasm_bindgen::from_value::<LangResponse>(payload) else {
            return;
        };
        if let LangResponse::Diagnostics { diagnostics, .. } = message {
            state.diagnostics.set(diagnostics);
        }
    });
    worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    Lang { worker }
}

/// Seeds the worker with the command and standard-library vocabulary.
pub fn init(lang: &Lang, commands: Vec<CommandInfo>, stdlib: Vec<StdModule>) {
    send(lang, &LangRequest::Init { commands, stdlib });
}

/// Requests a compile-check of the given source.
pub fn check(lang: &Lang, request_id: u32, source: String) {
    send(lang, &LangRequest::Check { request_id, source });
}

fn send(lang: &Lang, request: &LangRequest) {
    let envelope = js_sys::Object::new();
    let value = serde_wasm_bindgen::to_value(request).unwrap_or(JsValue::NULL);
    let _ = js_sys::Reflect::set(&envelope, &JsValue::from_str(MESSAGE_KEY), &value);
    let _ = lang.worker.post_message(&envelope);
}
