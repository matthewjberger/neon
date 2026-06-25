//! The page's hearsay peer: a WebSocket session against the broker the desktop
//! host runs, speaking the hearsay wire format directly (each client frame is a
//! postcard-encoded `PeerEvent`). A page served outside the shell has no broker,
//! so the socket never opens and everything stays inert. For now the peer only
//! publishes spawn requests, so any window can open another; subscribing for
//! workspace-layout coordination comes next.

use std::cell::RefCell;

use hearsay::PeerEvent;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{BinaryType, WebSocket};

const BROKER_WEBSOCKET_URL: &str = "ws://127.0.0.1:8783";
const SPAWN_TOPIC: &str = "neon/shell/request-spawn";
const RECONNECT_DELAY_MILLISECONDS: i32 = 2000;

struct Link {
    socket: WebSocket,
    client_id: String,
}

thread_local! {
    static LINK: RefCell<Option<Link>> = const { RefCell::new(None) };
}

/// The role and shell id the desktop shell passed through the page URL. A page
/// served outside the shell has neither and acts as a lone primary.
pub fn detect_shell() -> (bool, String) {
    let search = web_sys::window()
        .and_then(|window| window.location().search().ok())
        .unwrap_or_default();
    let mut is_primary = true;
    let mut shell_id = format!(
        "page-{:08x}",
        (js_sys::Math::random() * u32::MAX as f64) as u32
    );
    for pair in search.trim_start_matches('?').split('&') {
        if let Some(role) = pair.strip_prefix("role=") {
            is_primary = role != "child";
        }
        if let Some(shell) = pair.strip_prefix("shell=") {
            shell_id = shell.to_string();
        }
    }
    (is_primary, shell_id)
}

/// Opens the broker session and keeps reconnecting until a broker is listening,
/// so the page can connect before the host's listener is up.
pub fn start() {
    let (_is_primary, shell_id) = detect_shell();
    connect(shell_id);
}

fn connect(client_id: String) {
    let Ok(socket) = WebSocket::new(BROKER_WEBSOCKET_URL) else {
        schedule_reconnect(client_id);
        return;
    };
    socket.set_binary_type(BinaryType::Arraybuffer);

    let open_socket = socket.clone();
    let open_id = client_id.clone();
    let onopen = Closure::<dyn FnMut()>::new(move || {
        send_event(
            &open_socket,
            &PeerEvent::Hello {
                id: open_id.clone(),
            },
        );
    });
    socket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let reconnect_id = client_id.clone();
    let onclose = Closure::once_into_js(move || {
        LINK.with(|link| *link.borrow_mut() = None);
        schedule_reconnect(reconnect_id);
    });
    socket.set_onclose(Some(onclose.unchecked_ref()));

    LINK.with(|link| {
        *link.borrow_mut() = Some(Link { socket, client_id });
    });
}

/// Asks the host shell to open another window. The host owns the broker, so the
/// request rides it no matter which window publishes; a no-op with no broker.
pub fn request_spawn_window() {
    publish_text(SPAWN_TOPIC, "");
}

fn publish_text(topic: &str, payload: &str) {
    LINK.with(|link| {
        if let Some(link) = link.borrow().as_ref() {
            send_event(
                &link.socket,
                &PeerEvent::PublishText {
                    id: link.client_id.clone(),
                    topic: topic.to_string(),
                    payload: payload.to_string(),
                    local_only: false,
                },
            );
        }
    });
}

fn schedule_reconnect(client_id: String) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let callback = Closure::once_into_js(move || connect(client_id));
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.unchecked_ref(),
        RECONNECT_DELAY_MILLISECONDS,
    );
}

fn send_event(socket: &WebSocket, event: &PeerEvent) {
    if socket.ready_state() != WebSocket::OPEN {
        return;
    }
    if let Ok(frame) = postcard::to_allocvec(event) {
        let _ = socket.send_with_u8_array(&frame);
    }
}
