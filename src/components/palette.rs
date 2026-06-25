//! The command palette: a filterable, keyboard-driven list of editor commands,
//! opened with Ctrl+Shift+P (and by plugins). The same registry plugins invoke.

use leptos::html;
use leptos::prelude::*;

use crate::bridge::Bridge;
use crate::commands;
use crate::state::EditorState;

#[component]
pub fn Palette(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    state: EditorState,
) -> impl IntoView {
    let filter = RwSignal::new(String::new());
    let selected = RwSignal::new(0_usize);
    let input_ref = NodeRef::<html::Input>::new();

    Effect::new(move |_| {
        if state.editing.palette_open.get() {
            filter.set(String::new());
            selected.set(0);
            if let Some(input) = input_ref.get() {
                let _ = input.focus();
            }
        }
    });

    let filtered = move || {
        let needle = filter.get().to_lowercase();
        commands::palette_items(state)
            .into_iter()
            .filter(|(title, _)| needle.is_empty() || title.to_lowercase().contains(&needle))
            .collect::<Vec<_>>()
    };

    let run_at = move |index: usize| {
        if let Some((_, command)) = filtered().into_iter().nth(index) {
            commands::run(command, state, bridge);
            state.editing.palette_open.set(false);
        }
    };

    view! {
        <Show when=move || state.editing.palette_open.get() fallback=|| ()>
            <div class="palette-overlay" on:click=move |_| state.editing.palette_open.set(false)>
                <div class="palette" on:click=move |event| event.stop_propagation()>
                    <input
                        class="palette-input"
                        node_ref=input_ref
                        placeholder="Run a command"
                        prop:value=move || filter.get()
                        on:input=move |event| {
                            filter.set(event_target_value(&event));
                            selected.set(0);
                        }
                        on:keydown=move |event| {
                            match event.key().as_str() {
                                "Escape" => {
                                    event.prevent_default();
                                    state.editing.palette_open.set(false);
                                }
                                "Enter" => {
                                    event.prevent_default();
                                    run_at(selected.get_untracked());
                                }
                                "ArrowDown" => {
                                    event.prevent_default();
                                    let count = filtered().len().max(1);
                                    selected.update(|index| *index = (*index + 1) % count);
                                }
                                "ArrowUp" => {
                                    event.prevent_default();
                                    let count = filtered().len().max(1);
                                    selected.update(|index| *index = (*index + count - 1) % count);
                                }
                                _ => {}
                            }
                        }
                    />
                    <div class="palette-list">
                        <For
                            each=move || { filtered().into_iter().enumerate().collect::<Vec<_>>() }
                            key=|(index, (title, _))| (*index, title.clone())
                            children=move |(index, (title, _command))| {
                                view! {
                                    <div
                                        class="palette-item"
                                        class:active=move || selected.get() == index
                                        on:mouseenter=move |_| selected.set(index)
                                        on:click=move |_| run_at(index)
                                    >
                                        {title}
                                    </div>
                                }
                            }
                        />
                    </div>
                </div>
            </div>
        </Show>
    }
}
