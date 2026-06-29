//! The multibuffer view: excerpts gathered from across files into one scrollable
//! view, grouped by file. Each excerpt's line is editable in place; committing
//! (Enter or blur) splices the new text back into the source file's buffer and
//! notifies the language server. The line number jumps to the source.

use leptos::prelude::*;
use protocol::SearchHit;
use web_sys::KeyboardEvent;

use crate::state::{EditorState, PluginKind, basename};

#[component]
pub fn MultiBufferView(state: EditorState) -> impl IntoView {
    view! {
        <div class="multibuffer">
            {move || {
                let Some(buffer) = state.multibuffer.get() else {
                    return view! {
                        <div class="multibuffer-empty">"No multibuffer open"</div>
                    }
                    .into_any();
                };
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
                                let header = basename(&path).to_string();
                                let rows = hits
                                    .into_iter()
                                    .map(move |hit| excerpt_row(state, &path, hit))
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

/// One editable excerpt row: a jump-to-source line number and the line's text in
/// an input that writes back to the source file on Enter or blur.
fn excerpt_row(state: EditorState, path: &str, hit: SearchHit) -> AnyView {
    let line = hit.line;
    let jump_path = path.to_string();
    let commit_path = path.to_string();
    let blur_path = path.to_string();
    let jump = move |_| {
        crate::fs::read_file(&jump_path);
        state.explorer.goto.set(Some((jump_path.clone(), line)));
    };
    let on_keydown = move |event: KeyboardEvent| match event.key().as_str() {
        "Enter" => {
            event.prevent_default();
            commit_line(state, &commit_path, line, &event_target_value(&event));
        }
        "Escape" => event.prevent_default(),
        _ => {}
    };
    let on_blur = move |event| {
        commit_line(state, &blur_path, line, &event_target_value(&event));
    };
    view! {
        <div class="multibuffer-row">
            <span class="multibuffer-line" on:click=jump>{line}</span>
            <input
                class="multibuffer-edit"
                spellcheck="false"
                value=hit.text
                on:keydown=on_keydown
                on:blur=on_blur
            />
        </div>
    }
    .into_any()
}

/// Splices a replacement line back into a file's buffer and notifies the server.
fn commit_line(state: EditorState, path: &str, line: u32, new_text: &str) {
    let id = Some(path.to_string());
    let text = state.buffer_source(PluginKind::File, &id);
    let mut lines: Vec<&str> = text.split('\n').collect();
    let index = (line as usize).saturating_sub(1);
    if index >= lines.len() || lines[index] == new_text {
        return;
    }
    lines[index] = new_text;
    let updated = lines.join("\n");
    state.set_buffer_text(PluginKind::File, &id, updated);
    crate::lsp::did_change(state, path);
}
