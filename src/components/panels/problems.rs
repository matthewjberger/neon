//! The problems panel: every rust-analyzer diagnostic across open files, in one
//! list. Click a row to open the file and jump to the line.

use leptos::prelude::*;
use protocol::Severity;

use crate::state::{EditorState, basename};

#[component]
pub fn ProblemsPanel(state: EditorState) -> impl IntoView {
    view! {
        <Show when=move || state.lsp.problems_open.get() fallback=|| ()>
            <div class="lsp-log">
                <div class="lsp-log-header">
                    <span>
                        {move || format!("Problems ({})", state.lsp.problems.get().len())}
                    </span>
                    <button class="icon-button" on:click=move |_| state.lsp.problems_open.set(false)>
                        "x"
                    </button>
                </div>
                <div class="lsp-log-body">
                    <For
                        each=move || { state.lsp.problems.get().into_iter().enumerate().collect::<Vec<_>>() }
                        key=|(index, _)| *index
                        children=move |(_, (path, diagnostic))| {
                            let target = path.clone();
                            let line = diagnostic.line;
                            let location = format!("{}:{}", basename(&path), diagnostic.line);
                            let class = match diagnostic.severity {
                                Severity::Error => "problem-row problem-error",
                                Severity::Warning => "problem-row problem-warning",
                            };
                            view! {
                                <div
                                    class=class
                                    on:click=move |_| {
                                        crate::fs::read_file(&target);
                                        state.explorer.goto.set(Some((target.clone(), line)));
                                    }
                                >
                                    <span class="problem-loc">{location}</span>
                                    <span class="problem-message">{diagnostic.message}</span>
                                </div>
                            }
                        }
                    />
                </div>
            </div>
        </Show>
    }
}
