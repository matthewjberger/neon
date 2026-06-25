//! Project-wide search: a query over the workspace (run on the desktop, which
//! respects gitignore) and a results list. Clicking a hit opens the file and
//! jumps to the line.

use leptos::prelude::*;

use crate::fs;
use crate::state::{EditorState, basename};

#[component]
pub fn SearchPanel(state: EditorState) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let replacement = RwSignal::new(String::new());
    let run = move || {
        if let Some(root) = state.explorer.root.get_untracked() {
            fs::search(&root, &query.get_untracked());
        }
    };
    let replace = move || {
        if let Some(root) = state.explorer.root.get_untracked()
            && !query.get_untracked().is_empty()
        {
            fs::replace_all(&root, &query.get_untracked(), &replacement.get_untracked());
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
            <div class="search-replace-row">
                <input
                    class="search-input"
                    placeholder="Replace ($1 for groups)"
                    prop:value=move || replacement.get()
                    on:input=move |event| replacement.set(event_target_value(&event))
                />
                <button class="tool-button" on:click=move |_| replace()>"Replace all"</button>
            </div>
            <div class="search-results">
                {move || {
                    state.explorer.search_results
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
                                        state.explorer.goto.set(Some((path.clone(), line)));
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
