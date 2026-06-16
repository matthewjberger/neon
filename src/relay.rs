//! The page side of the agent bridge. Connects to the desktop MCP bridge's
//! relay websocket, receives `AgentRequest`s, and answers them. Editor-domain
//! requests (plugins, buffers, editor state) are answered here from the page
//! state. Scene-domain requests (commands, queries, screenshots) are forwarded
//! to the engine worker, which replies through the bridge's `WorkerMessage::Agent`
//! and back out via [`send_response`].

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use protocol::{AgentRequest, AgentResponse, ClientMessage};
use serde_json::json;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

use crate::bridge::{self, Bridge, send};
use crate::state::{EditorState, PluginKind};

const RELAY_URL: &str = "ws://127.0.0.1:8789";
const RECONNECT_MS: i32 = 1000;

/// The relay socket, shared between the connect logic and the bridge's worker
/// handler so scene-domain responses can be sent back to the MCP bridge.
pub type RelaySocket = Rc<RefCell<Option<WebSocket>>>;

/// Opens the relay and keeps it open, reconnecting on drop.
pub fn start(state: EditorState, bridge: Bridge, socket: RelaySocket) {
    connect(state, bridge, socket);
}

fn connect(state: EditorState, bridge: Bridge, socket: RelaySocket) {
    let Ok(websocket) = WebSocket::new(RELAY_URL) else {
        schedule_reconnect(state, bridge, socket);
        return;
    };

    let message_bridge = bridge.clone();
    let message_socket = socket.clone();
    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        if let Some(text) = event.data().as_string()
            && let Ok(request) = serde_json::from_str::<AgentRequest>(&text)
        {
            handle_request(state, &message_bridge, &message_socket, request);
        }
    });
    websocket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let close_bridge = bridge.clone();
    let close_socket = socket.clone();
    let onclose = Closure::<dyn FnMut()>::new(move || {
        schedule_reconnect(state, close_bridge.clone(), close_socket.clone());
    });
    websocket.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    *socket.borrow_mut() = Some(websocket);
}

fn schedule_reconnect(state: EditorState, bridge: Bridge, socket: RelaySocket) {
    *socket.borrow_mut() = None;
    let Some(window) = web_sys::window() else {
        return;
    };
    let callback = Closure::<dyn FnMut()>::new(move || {
        connect(state, bridge.clone(), socket.clone());
    });
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        RECONNECT_MS,
    );
    callback.forget();
}

fn handle_request(
    state: EditorState,
    bridge: &Bridge,
    socket: &RelaySocket,
    request: AgentRequest,
) {
    let is_scene = matches!(
        request,
        AgentRequest::RunCommand { .. }
            | AgentRequest::QueryScene { .. }
            | AgentRequest::Screenshot { .. }
    );
    if is_scene {
        send(bridge, &ClientMessage::Agent(Box::new(request)));
        return;
    }

    let response = match request {
        AgentRequest::GetEditorState { correlation_id } => AgentResponse::EditorState {
            correlation_id,
            state: editor_state_json(state),
        },
        AgentRequest::GetBuffer {
            correlation_id,
            buffer,
        } => AgentResponse::Buffer {
            correlation_id,
            text: buffer_text(state, buffer),
        },
        AgentRequest::SetBuffer {
            correlation_id,
            buffer,
            text,
        } => {
            set_buffer(state, bridge, &buffer, text);
            AgentResponse::Ok { correlation_id }
        }
        AgentRequest::ListPlugins { correlation_id } => AgentResponse::Plugins {
            correlation_id,
            plugins: state.plugins.get_untracked(),
        },
        AgentRequest::EditPlugin {
            correlation_id,
            plugin,
        } => {
            edit_plugin(state, bridge, plugin);
            AgentResponse::Ok { correlation_id }
        }
        _ => return,
    };
    send_response(socket, &response);
}

fn editor_state_json(state: EditorState) -> serde_json::Value {
    let plugins: Vec<serde_json::Value> = state
        .plugins
        .get_untracked()
        .into_iter()
        .map(|plugin| json!({ "id": plugin.id, "name": plugin.name, "enabled": plugin.enabled }))
        .collect();
    let selected = state
        .selected
        .get_untracked()
        .map(|detail| json!({ "id": detail.id, "name": detail.name }));
    json!({
        "plugins": plugins,
        "active": state.active_id(),
        "selected": selected,
        "running": state.running.get_untracked(),
        "entity_count": state.entity_count.get_untracked(),
        "theme": state.theme.get_untracked(),
    })
}

fn buffer_text(state: EditorState, buffer: Option<String>) -> String {
    match buffer {
        Some(id) => state.plugins.with_untracked(|plugins| {
            plugins
                .iter()
                .find(|plugin| plugin.id == id)
                .map(|plugin| plugin.source.clone())
                .unwrap_or_default()
        }),
        None => state.active_source(),
    }
}

fn set_buffer(state: EditorState, bridge: &Bridge, buffer: &str, text: String) {
    state.plugins.update(|plugins| {
        if let Some(plugin) = plugins.iter_mut().find(|plugin| plugin.id == buffer) {
            plugin.source = text;
        }
    });
    crate::plugins::save(&state.plugins.get_untracked());
    bridge::sync_plugins(bridge, state);
}

fn edit_plugin(state: EditorState, bridge: &Bridge, plugin: protocol::PluginSource) {
    state.plugins.update(|plugins| {
        if let Some(existing) = plugins.iter_mut().find(|existing| existing.id == plugin.id) {
            *existing = plugin.clone();
        } else {
            plugins.push(plugin.clone());
        }
    });
    state.open_in_focused(PluginKind::Scene, Some(plugin.id.clone()));
    crate::plugins::save(&state.plugins.get_untracked());
    bridge::sync_plugins(bridge, state);
}

/// Sends one agent response back to the bridge if the relay is open.
pub fn send_response(socket: &RelaySocket, response: &AgentResponse) {
    if let Some(websocket) = socket.borrow().as_ref()
        && websocket.ready_state() == WebSocket::OPEN
        && let Ok(text) = serde_json::to_string(response)
    {
        let _ = websocket.send_with_str(&text);
    }
}
