//! The command-and-event console: a live log of the plugin tick's traffic and a
//! field to submit one api `Command` as json.

use leptos::prelude::*;
use protocol::{ClientMessage, LogKind};

use crate::bridge::{Bridge, send};
use crate::state::EditorState;

#[component]
pub fn Console(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    state: EditorState,
) -> impl IntoView {
    let input = RwSignal::new(String::new());

    let submit = move |_| {
        let text = input.get_untracked().trim().to_string();
        if text.is_empty() {
            return;
        }
        if let Some(bridge) = bridge.get_value() {
            send(&bridge, &ClientMessage::SubmitCommand { command: text });
        }
    };

    view! {
        <div class="console">
            <div class="panel-title">"Console"</div>
            <div class="console-log">
                <For
                    each=move || { state.log.get().into_iter().enumerate().collect::<Vec<_>>() }
                    key=|(index, _)| *index
                    children=move |(_, entry)| {
                        view! {
                            <div class=log_class(entry.kind)>
                                <span class="log-label">{entry.label}</span>
                                <span class="log-detail">{entry.detail}</span>
                            </div>
                        }
                    }
                />
            </div>
            <div class="console-input-row">
                <input
                    class="console-input"
                    placeholder="{\"SpawnCube\":{\"position\":[0,0.5,0]}}"
                    prop:value=move || input.get()
                    on:input=move |event| input.set(event_target_value(&event))
                    on:keydown=move |event| {
                        if event.key() == "Enter" {
                            submit(());
                        }
                    }
                />
                <button class="tool-button" on:click=move |_| submit(())>
                    "Submit"
                </button>
            </div>
        </div>
    }
}

fn log_class(kind: LogKind) -> &'static str {
    match kind {
        LogKind::Command => "log-row log-command",
        LogKind::Event => "log-row log-event",
        LogKind::Error => "log-row log-error",
    }
}
