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
