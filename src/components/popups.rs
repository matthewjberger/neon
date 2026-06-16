//! The LSP popups: the completion menu and the hover card, both anchored at a
//! caret or pointer pixel position the LSP client computed.

use leptos::prelude::*;

use crate::state::EditorState;

#[component]
pub fn CompletionPopup(state: EditorState) -> impl IntoView {
    view! {
        <Show when=move || state.completion.get().is_some() fallback=|| ()>
            <div
                class="completion"
                style=move || {
                    let (x, y) = state
                        .completion
                        .get()
                        .map(|menu| (menu.x, menu.y))
                        .unwrap_or((0.0, 0.0));
                    format!("left: {x}px; top: {y}px")
                }
            >
                {move || {
                    let items = state.completion.get().map(|menu| menu.items).unwrap_or_default();
                    let active = state.completion_index.get();
                    items
                        .into_iter()
                        .enumerate()
                        .map(|(index, entry)| {
                            view! {
                                <div
                                    class="completion-item"
                                    class:active=index == active
                                    on:mousedown=move |event: web_sys::MouseEvent| {
                                        event.prevent_default();
                                        crate::lsp::accept_completion(state, index);
                                    }
                                >
                                    <span class="completion-kind">{entry.kind}</span>
                                    <span class="completion-label">{entry.label}</span>
                                    <span class="completion-detail">{entry.detail}</span>
                                </div>
                            }
                        })
                        .collect_view()
                }}
            </div>
        </Show>
    }
}

#[component]
pub fn HoverCardView(state: EditorState) -> impl IntoView {
    view! {
        <Show when=move || state.hover.get().is_some() fallback=|| ()>
            <div
                class="hover-card"
                style=move || {
                    let (x, y) = state
                        .hover
                        .get()
                        .map(|card| (card.x, card.y + 16.0))
                        .unwrap_or((0.0, 0.0));
                    format!("left: {x}px; top: {y}px")
                }
            >
                {move || state.hover.get().map(|card| card.text).unwrap_or_default()}
            </div>
        </Show>
    }
}
