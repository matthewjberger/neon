//! The Claude chat pane. Opening it asks the desktop shell to start the agent
//! bridge and chat relay, then connects to the relay websocket and renders the
//! stream-json the Claude subprocess emits. In a plain browser there is no shell,
//! so it stays in a reconnecting state. The MCP tools Claude calls drive the
//! editor through the agent bridge (see `DESIGN.md`).

use leptos::prelude::*;
use serde_json::Value;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

use crate::state::EditorState;

const CHAT_URL: &str = "ws://127.0.0.1:8791";
const RECONNECT_MS: i32 = 1000;
const TOOL_INPUT_LIMIT: usize = 160;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    User,
    Assistant,
    Thinking,
    Tool,
    Info,
    Error,
}

#[derive(Clone)]
struct ChatEntry {
    kind: EntryKind,
    text: String,
}

type ChatSocket = StoredValue<Option<WebSocket>, LocalStorage>;

#[component]
pub fn ChatPane(state: EditorState) -> impl IntoView {
    let messages = RwSignal::new(Vec::<ChatEntry>::new());
    let input = RwSignal::new(String::new());
    let connected = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let socket: ChatSocket = StoredValue::new_local(None);
    let started = StoredValue::new(false);

    Effect::new(move |_| {
        if !state.panels.chat.get() {
            return;
        }
        if !started.get_value() {
            started.set_value(true);
            crate::ipc::notify_host("open-chat");
            connect(socket, messages, connected, busy);
        }
    });

    let send_prompt = move || {
        let text = input.get_untracked().trim().to_string();
        if text.is_empty() {
            return;
        }
        let Some(websocket) = socket.get_value() else {
            return;
        };
        if websocket.ready_state() != WebSocket::OPEN {
            return;
        }
        let payload = serde_json::json!({ "type": "user", "text": text }).to_string();
        if websocket.send_with_str(&payload).is_ok() {
            messages.update(|entries| {
                entries.push(ChatEntry {
                    kind: EntryKind::User,
                    text,
                })
            });
            input.set(String::new());
            busy.set(true);
        }
    };

    view! {
        <div class="chat-pane" style:display=move || if state.panels.chat.get() { "flex" } else { "none" }>
            <div class="chat-header">
                <span class=move || {
                    if connected.get() { "chat-status connected" } else { "chat-status" }
                }></span>
                <span class="chat-title">"Claude"</span>
            </div>
            <div class="chat-messages">
                {move || {
                    messages
                        .get()
                        .into_iter()
                        .map(|entry| view! { <div class=entry_class(entry.kind)>{entry.text}</div> })
                        .collect_view()
                }}
                <Show when=move || busy.get() fallback=|| ()>
                    <div class="chat-entry info">"Working..."</div>
                </Show>
            </div>
            <div class="chat-compose">
                <textarea
                    class="chat-input"
                    placeholder=move || {
                        if connected.get() {
                            "Ask Claude to drive the editor"
                        } else {
                            "Connecting to Claude (desktop only)..."
                        }
                    }
                    prop:value=move || input.get()
                    on:input=move |event| input.set(event_target_value(&event))
                    on:keydown=move |event| {
                        if event.key() == "Enter" && !event.shift_key() {
                            event.prevent_default();
                            send_prompt();
                        }
                    }
                ></textarea>
                <button class="chat-send" on:click=move |_| send_prompt()>"Send"</button>
            </div>
        </div>
    }
}

fn connect(
    socket: ChatSocket,
    messages: RwSignal<Vec<ChatEntry>>,
    connected: RwSignal<bool>,
    busy: RwSignal<bool>,
) {
    let Ok(websocket) = WebSocket::new(CHAT_URL) else {
        schedule_reconnect(socket, messages, connected, busy);
        return;
    };

    let onopen = Closure::<dyn FnMut()>::new(move || connected.set(true));
    websocket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        if let Some(text) = event.data().as_string() {
            handle_event(&text, messages, busy);
        }
    });
    websocket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let onclose = Closure::<dyn FnMut()>::new(move || {
        connected.set(false);
        schedule_reconnect(socket, messages, connected, busy);
    });
    websocket.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    socket.set_value(Some(websocket));
}

fn schedule_reconnect(
    socket: ChatSocket,
    messages: RwSignal<Vec<ChatEntry>>,
    connected: RwSignal<bool>,
    busy: RwSignal<bool>,
) {
    socket.set_value(None);
    let Some(window) = web_sys::window() else {
        return;
    };
    let callback = Closure::<dyn FnMut()>::new(move || {
        connect(socket, messages, connected, busy);
    });
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        RECONNECT_MS,
    );
    callback.forget();
}

fn handle_event(text: &str, messages: RwSignal<Vec<ChatEntry>>, busy: RwSignal<bool>) {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return;
    };
    match value.get("type").and_then(Value::as_str) {
        Some("system") if value.get("subtype").and_then(Value::as_str) == Some("init") => {
            let model = value
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or("a model");
            push(
                messages,
                EntryKind::Info,
                format!("Session started ({model})"),
            );
        }
        Some("assistant") => {
            let Some(content) = value.pointer("/message/content").and_then(Value::as_array) else {
                return;
            };
            for block in content {
                match block.get("type").and_then(Value::as_str) {
                    Some("text") => {
                        if let Some(text) = block.get("text").and_then(Value::as_str)
                            && !text.trim().is_empty()
                        {
                            push(messages, EntryKind::Assistant, text.to_string());
                        }
                    }
                    Some("thinking") => {
                        if let Some(text) = block.get("thinking").and_then(Value::as_str)
                            && !text.trim().is_empty()
                        {
                            push(messages, EntryKind::Thinking, text.to_string());
                        }
                    }
                    Some("tool_use") => {
                        let name = block.get("name").and_then(Value::as_str).unwrap_or("tool");
                        let name = name.strip_prefix("mcp__neon__").unwrap_or(name);
                        let arguments = block
                            .get("input")
                            .map(|input| truncate(&input.to_string(), TOOL_INPUT_LIMIT))
                            .unwrap_or_default();
                        push(messages, EntryKind::Tool, format!("{name} {arguments}"));
                    }
                    _ => {}
                }
            }
        }
        Some("result") => {
            busy.set(false);
            if value
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                let text = value
                    .get("result")
                    .and_then(Value::as_str)
                    .unwrap_or("the turn failed");
                push(messages, EntryKind::Error, text.to_string());
            }
        }
        Some("stderr") => {
            if let Some(text) = value.get("text").and_then(Value::as_str) {
                push(messages, EntryKind::Error, text.to_string());
            }
        }
        _ => {}
    }
}

fn push(messages: RwSignal<Vec<ChatEntry>>, kind: EntryKind, text: String) {
    messages.update(|entries| entries.push(ChatEntry { kind, text }));
}

fn entry_class(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::User => "chat-entry user",
        EntryKind::Assistant => "chat-entry assistant",
        EntryKind::Thinking => "chat-entry thinking",
        EntryKind::Tool => "chat-entry tool",
        EntryKind::Info => "chat-entry info",
        EntryKind::Error => "chat-entry error",
    }
}

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        text.to_string()
    } else {
        let kept: String = text.chars().take(limit).collect();
        format!("{kept}...")
    }
}
