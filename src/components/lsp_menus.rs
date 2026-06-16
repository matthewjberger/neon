//! Two small LSP-driven overlays: the rename prompt (an input prefilled with the
//! symbol under the caret) and the code-action picker (a list of the actions the
//! server offers). Both apply through the language client.

use leptos::html;
use leptos::prelude::*;

use crate::state::EditorState;

#[component]
pub fn RenamePrompt(state: EditorState) -> impl IntoView {
    let text = RwSignal::new(String::new());
    let input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        if let Some(initial) = state.rename.get() {
            text.set(initial);
            if let Some(element) = input.get() {
                let _ = element.focus();
                element.select();
            }
        }
    });
    view! {
        <Show when=move || state.rename.get().is_some() fallback=|| ()>
            <div class="overlay-scrim" on:click=move |_| state.rename.set(None)>
                <div class="prompt-box" on:click=move |event| event.stop_propagation()>
                    <span class="prompt-label">"Rename symbol"</span>
                    <input
                        class="prompt-input"
                        node_ref=input
                        prop:value=move || text.get()
                        on:input=move |event| text.set(event_target_value(&event))
                        on:keydown=move |event: web_sys::KeyboardEvent| {
                            if event.key() == "Enter" {
                                crate::lsp::submit_rename(state, &text.get_untracked());
                            } else if event.key() == "Escape" {
                                state.rename.set(None);
                            }
                        }
                    />
                </div>
            </div>
        </Show>
    }
}

#[component]
pub fn SymbolPicker(state: EditorState) -> impl IntoView {
    let filter = RwSignal::new(String::new());
    let selected = RwSignal::new(0_usize);
    let input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        if !state.symbol_picker.get().is_empty() {
            filter.set(String::new());
            selected.set(0);
            if let Some(element) = input.get() {
                let _ = element.focus();
            }
        }
    });
    let filtered = move || {
        let needle = filter.get().to_lowercase();
        state
            .symbol_picker
            .get()
            .into_iter()
            .filter(|hit| needle.is_empty() || hit.text.to_lowercase().contains(&needle))
            .collect::<Vec<_>>()
    };
    let go = move |index: usize| {
        if let Some(hit) = filtered().into_iter().nth(index) {
            crate::fs::read_file(&hit.path);
            state.goto.set(Some((hit.path.clone(), hit.line)));
            state.symbol_picker.set(Vec::new());
        }
    };
    view! {
        <Show when=move || !state.symbol_picker.get().is_empty() fallback=|| ()>
            <div class="palette-overlay" on:click=move |_| state.symbol_picker.set(Vec::new())>
                <div class="palette" on:click=move |event| event.stop_propagation()>
                    <input
                        class="palette-input"
                        node_ref=input
                        placeholder="Jump to symbol"
                        prop:value=move || filter.get()
                        on:input=move |event| {
                            filter.set(event_target_value(&event));
                            selected.set(0);
                        }
                        on:keydown=move |event| {
                            match event.key().as_str() {
                                "Escape" => {
                                    event.prevent_default();
                                    state.symbol_picker.set(Vec::new());
                                }
                                "Enter" => {
                                    event.prevent_default();
                                    go(selected.get_untracked());
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
                            key=|(index, hit)| (*index, hit.line, hit.text.clone())
                            children=move |(index, hit)| {
                                let location = format!(
                                    "{}:{}",
                                    crate::state::basename(&hit.path),
                                    hit.line,
                                );
                                view! {
                                    <div
                                        class="palette-item"
                                        class:active=move || selected.get() == index
                                        on:mouseenter=move |_| selected.set(index)
                                        on:click=move |_| go(index)
                                    >
                                        <span class="palette-symbol">{hit.text.clone()}</span>
                                        <span class="palette-symbol-loc">{location}</span>
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

#[component]
pub fn CodeActionMenu(state: EditorState) -> impl IntoView {
    view! {
        <Show when=move || !state.code_actions.get().is_empty() fallback=|| ()>
            <div class="overlay-scrim" on:click=move |_| state.code_actions.set(Vec::new())>
                <div class="prompt-box" on:click=move |event| event.stop_propagation()>
                    <span class="prompt-label">"Code actions"</span>
                    {move || {
                        state
                            .code_actions
                            .get()
                            .into_iter()
                            .enumerate()
                            .map(|(index, title)| {
                                view! {
                                    <div
                                        class="prompt-item"
                                        on:click=move |_| crate::lsp::apply_code_action(state, index)
                                    >
                                        {title}
                                    </div>
                                }
                            })
                            .collect_view()
                    }}
                </div>
            </div>
        </Show>
    }
}
