//! The bottom status bar: the focused buffer's path, its dirty state, its
//! language, and the language-server status (click to toggle the LSP log).

use leptos::prelude::*;

use crate::state::{EditorState, PluginKind, language_for_path};

#[component]
pub fn StatusBar(state: EditorState) -> impl IntoView {
    view! {
        <div class="status-bar">
            <span class="status-item">
                {move || {
                    let buffer = state.focused_buffer();
                    match buffer.kind {
                        PluginKind::File => buffer.id.clone().unwrap_or_default(),
                        _ => state.buffer_name(buffer.kind, &buffer.id),
                    }
                }}
            </span>
            <Show
                when=move || {
                    let buffer = state.focused_buffer();
                    state.is_dirty(buffer.kind, &buffer.id)
                }
                fallback=|| ()
            >
                <span class="status-dirty">"unsaved"</span>
            </Show>
            <span class="status-spacer"></span>
            <span class="status-item">{move || language_label(state)}</span>
            <span
                class="status-item lsp"
                class:active=move || state.lsp_started.get()
                title="Toggle the rust-analyzer log"
                on:click=move |_| state.lsp_log_open.update(|open| *open = !*open)
            >
                {move || if state.lsp_started.get() { "rust-analyzer" } else { "LSP off" }}
            </span>
        </div>
    }
}

fn language_label(state: EditorState) -> &'static str {
    let buffer = state.focused_buffer();
    match buffer.kind {
        PluginKind::File => buffer
            .id
            .as_deref()
            .map(language_for_path)
            .unwrap_or("plaintext"),
        PluginKind::Builtin | PluginKind::Scene | PluginKind::Editor => "rhai",
    }
}
