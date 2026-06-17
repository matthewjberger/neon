//! Renders the extra carets for multi-cursor mode as thin overlays at each
//! offset, positioned with the shared caret geometry and repositioned on scroll.

use leptos::prelude::*;

use crate::state::EditorState;

#[component]
pub fn MultiCursorOverlay(state: EditorState) -> impl IntoView {
    view! {
        <Show when=move || !state.cursors.get().is_empty() fallback=|| ()>
            {move || {
                state.editor_scroll.get();
                let cursors = state.cursors.get();
                let Some(element) = crate::components::find::active() else {
                    return ().into_any();
                };
                let value = element.value();
                let height = crate::caret::line_height(&element);
                cursors
                    .into_iter()
                    .map(|offset| {
                        let (line, column) = crate::multicursor::line_col(&value, offset);
                        let (x, y) = crate::caret::cell(&element, line, column);
                        let style = format!("left:{x}px;top:{y}px;height:{height}px");
                        view! { <div class="extra-caret" style=style></div> }
                    })
                    .collect_view()
                    .into_any()
            }}
        </Show>
    }
}
