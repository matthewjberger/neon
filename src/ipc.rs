//! The webview IPC channel to the desktop shell. Sending a signal here starts a
//! desktop-side bridge (the agent, chat, filesystem, or language-server relay).
//! A no-op in a plain browser, where there is no shell.

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// Posts a message to the desktop shell over the webview IPC channel.
pub fn notify_host(message: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(ipc) = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("ipc")) else {
        return;
    };
    if ipc.is_undefined() || ipc.is_null() {
        return;
    }
    let Ok(post) = js_sys::Reflect::get(&ipc, &JsValue::from_str("postMessage")) else {
        return;
    };
    if let Ok(function) = post.dyn_into::<js_sys::Function>() {
        let _ = function.call1(&ipc, &JsValue::from_str(message));
    }
}
