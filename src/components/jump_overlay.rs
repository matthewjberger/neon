//! The jump overlay: while jump mode is active it dims the buffer and shows a
//! label at every target. The part of the label already typed is dimmed and the
//! next key, the sentinel to press, is highlighted. Only labels still matching
//! the typed prefix are shown.

use leptos::prelude::*;

use crate::state::EditorState;

#[component]
pub fn JumpOverlay(state: EditorState) -> impl IntoView {
    view! {
        <Show when=move || state.editing.jump.get().is_some() fallback=|| ()>
            <div class="jump-overlay">
                {move || {
                    let jump = state.editing.jump.get();
                    let pending = jump.as_ref().map(|jump| jump.pending.clone()).unwrap_or_default();
                    let typed_len = pending.chars().count();
                    jump.map(|jump| jump.targets)
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|target| target.label.starts_with(&pending))
                        .map(|target| {
                            let typed: String = target.label.chars().take(typed_len).collect();
                            let rest: String = target.label.chars().skip(typed_len).collect();
                            view! {
                                <div
                                    class="jump-label"
                                    style=format!("left: {}px; top: {}px", target.x, target.y)
                                >
                                    <span class="jump-typed">{typed}</span>
                                    <span class="jump-key">{rest}</span>
                                </div>
                            }
                        })
                        .collect_view()
                }}
            </div>
        </Show>
    }
}
