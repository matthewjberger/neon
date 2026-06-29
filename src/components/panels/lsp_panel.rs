//! The language-server surface: a consent toast that gates starting the server
//! (it spawns a process), and a log panel showing the server's output.

use leptos::prelude::*;

use crate::state::{EditorState, basename, lsp_server_name};

#[component]
pub fn LspConsent(state: EditorState) -> impl IntoView {
    view! {
        <Show when=move || state.lsp.consent.get() fallback=|| ()>
            <div class="consent-toast">
                <span class="consent-text">
                    {move || {
                        let root = state.explorer.root
                            .get()
                            .map(|root| basename(&root).to_string())
                            .unwrap_or_else(|| "this folder".to_string());
                        let server = state.lsp.language
                            .get()
                            .map(|family| lsp_server_name(&family))
                            .unwrap_or("the language server");
                        format!("Start {server} for {root}?")
                    }}
                </span>
                <button class="tool-button" on:click=move |_| crate::lsp::enable(state)>
                    "Allow"
                </button>
                <button class="tool-button" on:click=move |_| state.lsp.consent.set(false)>
                    "Dismiss"
                </button>
            </div>
        </Show>
    }
}

#[component]
pub fn LspLog(state: EditorState) -> impl IntoView {
    view! {
        <Show when=move || state.lsp.log_open.get() fallback=|| ()>
            <div class="lsp-log">
                <div class="lsp-log-header">
                    <span>"rust-analyzer log"</span>
                    <button class="icon-button" on:click=move |_| state.lsp.log_open.set(false)>
                        "x"
                    </button>
                </div>
                <div class="lsp-log-body">
                    {move || {
                        state.lsp.log
                            .get()
                            .into_iter()
                            .map(|line| view! { <div class="lsp-log-line">{line}</div> })
                            .collect_view()
                    }}
                </div>
            </div>
        </Show>
    }
}
