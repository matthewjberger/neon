//! Project-wide search: a query over the workspace (run on the desktop, which
//! respects gitignore) and a results list. Clicking a hit opens the file and
//! jumps to the line.

use leptos::prelude::*;

use crate::fs;
use crate::state::{EditorState, basename};

#[component]
pub fn SearchPanel(state: EditorState) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let run = move || {
        if let Some(root) = state.workspace_root.get_untracked() {
            fs::search(&root, &query.get_untracked());
        }
    };

    view! {
        <div class="search-panel">
            <div class="panel-title">
                <span>"Search"</span>
            </div>
            <input
                class="search-input"
                placeholder="Search project (regex)"
                prop:value=move || query.get()
                on:input=move |event| query.set(event_target_value(&event))
                on:keydown=move |event| {
                    if event.key() == "Enter" {
                        run();
                    }
                }
            />
            <div class="search-results">
                {move || {
                    state
                        .search_results
                        .get()
                        .into_iter()
                        .map(|hit| {
                            let path = hit.path.clone();
                            let line = hit.line;
                            let location = format!("{}:{}", basename(&hit.path), hit.line);
                            view! {
                                <div
                                    class="search-hit"
                                    on:click=move |_| {
                                        fs::read_file(&path);
                                        state.goto.set(Some((path.clone(), line)));
                                    }
                                >
                                    <span class="search-hit-loc">{location}</span>
                                    <span class="search-hit-text">{hit.text}</span>
                                </div>
                            }
                        })
                        .collect_view()
                }}
            </div>
        </div>
    }
}
