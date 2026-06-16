//! The control panel: a master surface for dispatching any editor command,
//! firing scene-api presets built from the live command manifest, and watching
//! the unified api log. The engine core only accepts commands and emits events,
//! so this exercises that surface and shows every call as it happens.

use leptos::prelude::*;
use protocol::{ClientMessage, CommandInfo, LogKind};

use crate::bridge::{Bridge, send};
use crate::commands::{self, palette_items};
use crate::state::EditorState;

#[component]
pub fn ControlPanel(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    state: EditorState,
) -> impl IntoView {
    view! {
        <Show when=move || state.control_panel_open.get() fallback=|| ()>
            <div class="control-panel">
                <div class="control-panel-header">
                    <span>"Control Panel"</span>
                    <button
                        class="icon-button"
                        on:click=move |_| state.control_panel_open.set(false)
                    >
                        "x"
                    </button>
                </div>
                <div class="control-panel-body">
                    <div class="control-section-title">"Editor commands"</div>
                    <div class="control-grid">
                        {move || {
                            palette_items(state)
                                .into_iter()
                                .map(|(label, command)| {
                                    view! {
                                        <button
                                            class="control-button"
                                            on:click=move |_| {
                                                commands::run(command.clone(), state, bridge);
                                            }
                                        >
                                            {label}
                                        </button>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                    <div class="control-section-title">"Scene api"</div>
                    <div class="control-grid">
                        <button
                            class="control-button"
                            on:click=move |_| submit(bridge, state, "{\"SpawnCube\":{\"position\":[0.0,0.5,0.0]}}")
                        >
                            "Spawn cube"
                        </button>
                        <button
                            class="control-button"
                            on:click=move |_| {
                                if let Some(bridge) = bridge.get_value() {
                                    state.log_api(LogKind::Command, "scene", "ResetScene");
                                    send(&bridge, &ClientMessage::ResetScene);
                                }
                            }
                        >
                            "Reset scene"
                        </button>
                        <button
                            class="control-button"
                            on:click=move |_| {
                                let running = !state.running.get_untracked();
                                state.running.set(running);
                                if let Some(bridge) = bridge.get_value() {
                                    state.log_api(LogKind::Command, "scene", format!("SetRunning({running})"));
                                    send(&bridge, &ClientMessage::SetRunning { running });
                                }
                            }
                        >
                            {move || if state.running.get() { "Pause runtime" } else { "Run runtime" }}
                        </button>
                        {move || {
                            state
                                .commands
                                .get()
                                .into_iter()
                                .map(|info| {
                                    let json = sample_command(&info);
                                    let label = info.variant.clone();
                                    view! {
                                        <button
                                            class="control-button"
                                            title=info.description.clone()
                                            on:click=move |_| submit(bridge, state, &json)
                                        >
                                            {label}
                                        </button>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                </div>
                <div class="control-log-header">
                    <span>"API log"</span>
                    <button
                        class="icon-button"
                        on:click=move |_| state.log.set(Vec::new())
                    >
                        "Clear"
                    </button>
                </div>
                <div class="control-log">
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
            </div>
        </Show>
    }
}

fn submit(bridge: StoredValue<Option<Bridge>, LocalStorage>, state: EditorState, command: &str) {
    if let Some(bridge) = bridge.get_value() {
        state.log_api(LogKind::Command, "scene", command.to_string());
        send(
            &bridge,
            &ClientMessage::SubmitCommand {
                command: command.to_string(),
            },
        );
    }
}

/// Builds a sample json command for a manifest entry: an externally tagged enum,
/// a bare string for a unit variant, an object of type-defaulted fields
/// otherwise.
fn sample_command(info: &CommandInfo) -> String {
    if info.fields.is_empty() {
        return format!("\"{}\"", info.variant);
    }
    let fields = info
        .fields
        .iter()
        .map(|field| format!("\"{}\":{}", field.name, default_value(&field.type_name)))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"{}\":{{{}}}}}", info.variant, fields)
}

fn default_value(type_name: &str) -> &'static str {
    let lowered = type_name.to_lowercase();
    if lowered.contains("vec4") || lowered.contains("color") || lowered.contains("quat") {
        "[1.0,1.0,1.0,1.0]"
    } else if lowered.contains("vec3") {
        "[0.0,0.5,0.0]"
    } else if lowered.contains("vec2") {
        "[0.0,0.0]"
    } else if lowered.contains("bool") {
        "false"
    } else if lowered.contains("f32") || lowered.contains("f64") {
        "0.5"
    } else if lowered.contains("string") || lowered.contains("str") {
        "\"\""
    } else if lowered.contains("u8")
        || lowered.contains("u16")
        || lowered.contains("u32")
        || lowered.contains("u64")
        || lowered.contains("usize")
        || lowered.contains("i8")
        || lowered.contains("i16")
        || lowered.contains("i32")
        || lowered.contains("i64")
        || lowered.contains("isize")
        || lowered.contains("entity")
    {
        "0"
    } else {
        "null"
    }
}

fn log_class(kind: LogKind) -> &'static str {
    match kind {
        LogKind::Command => "log-row log-command",
        LogKind::Event => "log-row log-event",
        LogKind::Error => "log-row log-error",
    }
}
