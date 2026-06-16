//! The which-key panel: the bottom strip that appears while a leader prefix is
//! pending, listing the next keys and what they do. The content is whatever the
//! active editor plugin published through `ShowMenu`, so the keymap stays in the
//! plugin and the page only renders it.

use leptos::prelude::*;

use crate::state::EditorState;

#[component]
pub fn WhichKey(state: EditorState) -> impl IntoView {
    view! {
        <Show when=move || state.leader.get().is_some() fallback=|| ()>
            <div class="whichkey">
                <div class="whichkey-title">
                    {move || state.leader.get().map(|menu| menu.title).unwrap_or_default()}
                </div>
                <div class="whichkey-grid">
                    {move || {
                        state
                            .leader
                            .get()
                            .map(|menu| menu.items)
                            .unwrap_or_default()
                            .into_iter()
                            .map(|item| {
                                let submenu = item.label.starts_with('+');
                                view! {
                                    <div class="whichkey-item">
                                        <span class="whichkey-key">{item.key}</span>
                                        <span class="whichkey-arrow">"\u{2192}"</span>
                                        <span class="whichkey-label" class:submenu=submenu>
                                            {item.label}
                                        </span>
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
