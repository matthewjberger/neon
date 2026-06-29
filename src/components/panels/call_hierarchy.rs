//! The call-hierarchy panel: the callers or callees of the symbol the caret was
//! on, from the language server. The header flips direction (re-querying at the
//! caret); clicking a row jumps to the related symbol.

use leptos::prelude::*;

use crate::state::EditorState;

#[component]
pub fn CallHierarchy(state: EditorState) -> impl IntoView {
    let title = move || {
        if state.lsp.call_hierarchy_incoming.get() {
            "Callers"
        } else {
            "Callees"
        }
    };
    let flip = move |_| {
        let incoming = state.lsp.call_hierarchy_incoming.get_untracked();
        crate::lsp::request_call_hierarchy(state, !incoming);
    };
    view! {
        <Show when=move || state.panels.call_hierarchy.get() fallback=|| ()>
            <div class="call-hierarchy">
                <div class="call-hierarchy-header">
                    <span>{title}</span>
                    <button class="call-hierarchy-flip" title="Flip direction" on:click=flip>
                        "\u{21c4}"
                    </button>
                    <button
                        class="call-hierarchy-close"
                        on:click=move |_| state.panels.call_hierarchy.set(false)
                    >
                        "\u{00d7}"
                    </button>
                </div>
                <div class="call-hierarchy-list">
                    {move || {
                        let entries = state.lsp.call_hierarchy.get();
                        if entries.is_empty() {
                            view! { <div class="call-hierarchy-empty">"No calls"</div> }.into_any()
                        } else {
                            entries
                                .into_iter()
                                .map(|entry| {
                                    let path = entry.path.clone();
                                    let line = entry.line;
                                    view! {
                                        <div
                                            class="call-hierarchy-row"
                                            on:click=move |_| {
                                                crate::fs::read_file(&path);
                                                state.explorer.goto.set(Some((path.clone(), line)));
                                            }
                                        >
                                            <span class="call-hierarchy-name">{entry.name}</span>
                                            <span class="call-hierarchy-detail">{entry.detail}</span>
                                        </div>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }
                    }}
                </div>
            </div>
        </Show>
    }
}
