//! The multibuffer view: excerpts gathered from across files into one scrollable
//! view, grouped by file. Each excerpt is a syntax-highlighted line that jumps to
//! its source. This is the navigable foundation; editing the excerpts in place is
//! layered on top of it.

use std::collections::HashSet;

use leptos::prelude::*;
use protocol::SearchHit;

use crate::highlight::highlight;
use crate::state::{EditorState, basename, language_for_path};

#[component]
pub fn MultiBufferView(state: EditorState) -> impl IntoView {
    let command_set = Memo::new(move |_| {
        state
            .commands
            .get()
            .into_iter()
            .map(|command| command.method)
            .collect::<HashSet<String>>()
    });
    view! {
        <div class="multibuffer">
            {move || {
                let Some(buffer) = state.multibuffer.get() else {
                    return view! {
                        <div class="multibuffer-empty">"No multibuffer open"</div>
                    }
                    .into_any();
                };
                let set = command_set.get();
                let mut groups: Vec<(String, Vec<SearchHit>)> = Vec::new();
                for hit in buffer.excerpts {
                    match groups.last_mut() {
                        Some((path, hits)) if *path == hit.path => hits.push(hit),
                        _ => groups.push((hit.path.clone(), vec![hit])),
                    }
                }
                view! {
                    <div class="multibuffer-title">{buffer.title}</div>
                    <div class="multibuffer-body">
                        {groups
                            .into_iter()
                            .map(|(path, hits)| {
                                let language = language_for_path(&path);
                                let header = basename(&path).to_string();
                                let rows = hits
                                    .into_iter()
                                    .map(|hit| {
                                        let runs = highlight(&hit.text, language, &set);
                                        let target = path.clone();
                                        let line = hit.line;
                                        let jump = move |_| {
                                            crate::fs::read_file(&target);
                                            state.explorer.goto.set(Some((target.clone(), line)));
                                        };
                                        view! {
                                            <div class="multibuffer-row" on:click=jump>
                                                <span class="multibuffer-line">{hit.line}</span>
                                                <span class="multibuffer-code">
                                                    {runs
                                                        .into_iter()
                                                        .map(|(class, run)| {
                                                            view! { <span class=class>{run}</span> }
                                                        })
                                                        .collect_view()}
                                                </span>
                                            </div>
                                        }
                                    })
                                    .collect_view();
                                view! {
                                    <div class="multibuffer-group">
                                        <div class="multibuffer-header">{header}</div>
                                        {rows}
                                    </div>
                                }
                            })
                            .collect_view()}
                    </div>
                }
                .into_any()
            }}
        </div>
    }
}
