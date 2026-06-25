//! The diagnostics strip under the focused pane's editor.

use leptos::prelude::*;

use crate::state::EditorState;

/// The diagnostics strip under the editor, shown only for the focused pane so a
/// split does not stack a strip per pane.
#[component]
pub(super) fn DiagnosticStrip(state: EditorState, pane_key: usize) -> impl IntoView {
    view! {
        <Show when=move || state.focused_key.get() == pane_key fallback=|| ()>
            <div class="diagnostics">
                <For
                    each=move || { state.diagnostics.get().into_iter().enumerate().collect::<Vec<_>>() }
                    key=|(index, _)| *index
                    children=move |(_, diag)| {
                        view! {
                            <div class="diagnostic">
                                <span class="diag-pos">
                                    {format!("{}:{}", diag.line, diag.column)}
                                </span>
                                {diag.message}
                            </div>
                        }
                    }
                />
            </div>
        </Show>
    }
}
