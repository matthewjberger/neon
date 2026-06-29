//! The undo-tree visualizer: a side panel listing the focused buffer's history
//! as a depth-indented tree, newest branch first, with the live state marked.
//! Clicking a node restores it, so a discarded redo branch is one click away.

use leptos::prelude::*;

use crate::state::EditorState;

#[component]
pub fn UndoTree(state: EditorState) -> impl IntoView {
    // Track every buffer store so the tree repaints on any edit while it is open.
    let rows = move || {
        state.files.track();
        state.plugins.track();
        state.editor_plugins.track();
        crate::undo::rows(state)
    };
    view! {
        <Show when=move || state.panels.undo_tree.get() fallback=|| ()>
            <div class="undo-tree">
                <div class="undo-tree-header">
                    <span>"Undo tree"</span>
                    <button
                        class="undo-tree-close"
                        on:click=move |_| state.panels.undo_tree.set(false)
                    >
                        "\u{00d7}"
                    </button>
                </div>
                <div class="undo-tree-list">
                    {move || {
                        rows()
                            .into_iter()
                            .map(|row| {
                                let index = row.index;
                                let indent = format!("padding-left: {}px;", 8 + row.depth * 16);
                                let mark = if row.current { "\u{25cf}" } else { "\u{25cb}" };
                                view! {
                                    <div
                                        class="undo-row"
                                        class:current=row.current
                                        style=indent
                                        on:click=move |_| crate::undo::restore(state, index)
                                    >
                                        <span class="undo-row-mark">{mark}</span>
                                        <span class="undo-row-preview">{row.preview}</span>
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
