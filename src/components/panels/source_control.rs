//! The source-control panel: the repo's branch, a commit-message box, and the
//! changed files split into staged and unstaged. Clicking a staged file unstages
//! it, an unstaged file stages it; the relay's reply refreshes the list.

use leptos::prelude::*;

use crate::state::{EditorState, basename};

#[component]
pub fn SourceControl(state: EditorState) -> impl IntoView {
    let message = RwSignal::new(String::new());
    let files = move || state.git_status.get().1;
    view! {
        <Show when=move || state.panels.git.get() fallback=|| ()>
            <div class="scm-panel">
                <div class="scm-header">
                    <span>
                        {move || {
                            let branch = state.git_status.with(|status| status.0.clone());
                            if branch.is_empty() {
                                "Source control".to_string()
                            } else {
                                format!("Source control \u{00b7} {branch}")
                            }
                        }}
                    </span>
                    <button class="scm-close" on:click=move |_| state.panels.git.set(false)>
                        "\u{00d7}"
                    </button>
                </div>
                <div class="scm-commit">
                    <input
                        class="scm-input"
                        prop:value=move || message.get()
                        on:input=move |event| message.set(event_target_value(&event))
                        placeholder="Commit message"
                    />
                    <button
                        class="scm-commit-btn"
                        on:click=move |_| {
                            let text = message.get_untracked();
                            if !text.trim().is_empty() {
                                crate::git::commit(state, &text);
                                message.set(String::new());
                            }
                        }
                    >
                        "Commit"
                    </button>
                </div>
                <div class="scm-list">
                    {move || {
                        let staged: Vec<_> = files().into_iter().filter(|file| file.staged).collect();
                        if staged.is_empty() {
                            return ().into_any();
                        }
                        view! {
                            <div class="scm-section">"Staged"</div>
                            {staged
                                .into_iter()
                                .map(|file| {
                                    let path = file.path.clone();
                                    let name = basename(&file.path).to_string();
                                    view! {
                                        <div
                                            class="scm-row"
                                            on:click=move |_| crate::git::unstage(state, &path)
                                        >
                                            <span class="scm-status">{file.status.clone()}</span>
                                            <span class="scm-path">{name}</span>
                                            <span class="scm-action">"\u{2212}"</span>
                                        </div>
                                    }
                                })
                                .collect_view()}
                        }
                            .into_any()
                    }}
                    {move || {
                        let changes: Vec<_> = files()
                            .into_iter()
                            .filter(|file| !file.staged)
                            .collect();
                        if changes.is_empty() {
                            return ().into_any();
                        }
                        view! {
                            <div class="scm-section">"Changes"</div>
                            {changes
                                .into_iter()
                                .map(|file| {
                                    let path = file.path.clone();
                                    let name = basename(&file.path).to_string();
                                    view! {
                                        <div
                                            class="scm-row"
                                            on:click=move |_| crate::git::stage(state, &path)
                                        >
                                            <span class="scm-status">{file.status.clone()}</span>
                                            <span class="scm-path">{name}</span>
                                            <span class="scm-action">"+"</span>
                                        </div>
                                    }
                                })
                                .collect_view()}
                        }
                            .into_any()
                    }}
                </div>
            </div>
        </Show>
    }
}
