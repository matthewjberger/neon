//! The top bar: run/pause, reset, the renderer stats, the theme picker, and the
//! Claude toggle. All actions are sends to the worker or signal writes; no state
//! lives here.

use leptos::prelude::*;
use protocol::ClientMessage;

use crate::bridge::{Bridge, send};
use crate::state::EditorState;
use crate::theme::{self, THEMES};

#[component]
pub fn Toolbar(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    state: EditorState,
) -> impl IntoView {
    let toggle_running = move |_| {
        let running = !state.running.get_untracked();
        state.running.set(running);
        if let Some(bridge) = bridge.get_value() {
            send(&bridge, &ClientMessage::SetRunning { running });
        }
    };

    let reset = move |_| {
        if let Some(bridge) = bridge.get_value() {
            send(&bridge, &ClientMessage::ResetScene);
        }
    };

    let toggle_reference = move |_| state.reference_open.update(|open| *open = !*open);
    let toggle_chat = move |_| state.chat_open.update(|open| *open = !*open);

    let theme_open = RwSignal::new(false);

    view! {
        <div class="toolbar">
            <span class="brand">"Neon"</span>
            <button class="tool-button" on:click=toggle_running>
                {move || if state.running.get() { "Pause" } else { "Run" }}
            </button>
            <button class="tool-button" on:click=reset>"Reset"</button>
            <button
                class="tool-button"
                class:active=move || state.viewport_open.get()
                title="Show or hide the 3D preview"
                on:click=move |_| state.viewport_open.update(|open| *open = !*open)
            >
                "Preview"
            </button>
            <button
                class="tool-button"
                class:active=move || state.console_open.get()
                title="Show or hide the console"
                on:click=move |_| state.console_open.update(|open| *open = !*open)
            >
                "Console"
            </button>
            <button
                class="tool-button"
                class:active=move || state.reference_open.get()
                on:click=toggle_reference
            >
                "Reference"
            </button>
            <button
                class="tool-button"
                class:active=move || state.control_panel_open.get()
                title="Dispatch any command and watch the api log"
                on:click=move |_| state.control_panel_open.update(|open| *open = !*open)
            >
                "Control Panel"
            </button>
            <span class="toolbar-spacer"></span>
            <Show when=move || state.editor_plugins.get().iter().any(|plugin| plugin.enabled) fallback=|| ()>
                <span class="stat mode-chip">{move || state.editor_mode.get()}</span>
            </Show>
            <Show when=move || !state.status.get().is_empty() fallback=|| ()>
                <span class="stat">{move || state.status.get()}</span>
            </Show>
            <span class="stat">{move || format!("{:.0} fps", state.fps.get())}</span>
            <span class="stat">{move || format!("{} entities", state.entity_count.get())}</span>
            <div class="theme-picker">
                <button
                    class="tool-button theme-button"
                    on:click=move |_| theme_open.update(|open| *open = !*open)
                >
                    {move || theme::theme_label(&state.theme.get())}
                    " \u{25be}"
                </button>
                <Show when=move || theme_open.get() fallback=|| ()>
                    <div
                        class="theme-menu"
                        on:mouseleave=move |_| theme::preview_theme(&state.theme.get_untracked())
                    >
                        <For each=move || THEMES.to_vec() key=|(id, _)| id.to_string() let:item>
                            {
                                let id = item.0;
                                let label = item.1;
                                view! {
                                    <div
                                        class="theme-option"
                                        class:active=move || state.theme.get() == id
                                        on:mouseenter=move |_| theme::preview_theme(id)
                                        on:click=move |_| {
                                            state.theme.set(id.to_string());
                                            theme_open.set(false);
                                        }
                                    >
                                        {label}
                                    </div>
                                }
                            }
                        </For>
                    </div>
                </Show>
            </div>
            <button
                class="tool-button claude-toggle"
                class:active=move || state.chat_open.get()
                title="Ask Claude to drive the editor"
                on:click=toggle_chat
            >
                "✦ Claude"
            </button>
        </div>
    }
}
