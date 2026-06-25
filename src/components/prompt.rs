//! A small text prompt for file operations: new file, rename, and delete
//! confirmation. Enter runs the action through the filesystem bridge, Escape
//! cancels. It reuses the overlay styling from the LSP prompts.

use leptos::html;
use leptos::prelude::*;

use crate::state::{EditorState, PromptAction};

#[component]
pub fn PromptView(state: EditorState) -> impl IntoView {
    let text = RwSignal::new(String::new());
    let input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        if let Some(prompt) = state.editing.prompt.get() {
            text.set(prompt.value);
            if let Some(element) = input.get() {
                let _ = element.focus();
                element.select();
            }
        }
    });
    let confirm = move || {
        let Some(prompt) = state.editing.prompt.get_untracked() else {
            return;
        };
        state.editing.prompt.set(None);
        let value = text.get_untracked();
        let value = value.trim();
        match prompt.action {
            PromptAction::CreateFile { dir } => {
                if !value.is_empty() {
                    crate::fs::create_path(&join(&dir, value));
                }
            }
            PromptAction::RenameEntry { from } => {
                if !value.is_empty() {
                    crate::fs::rename_path(&from, &join(&parent_of(&from), value));
                }
            }
            PromptAction::DeleteEntry { path } => crate::fs::delete_path(&path),
        }
    };
    view! {
        <Show when=move || state.editing.prompt.get().is_some() fallback=|| ()>
            <div class="overlay-scrim" on:click=move |_| state.editing.prompt.set(None)>
                <div class="prompt-box" on:click=move |event| event.stop_propagation()>
                    <span class="prompt-label">
                        {move || state.editing.prompt.get().map(|prompt| prompt.title).unwrap_or_default()}
                    </span>
                    <input
                        class="prompt-input"
                        node_ref=input
                        prop:value=move || text.get()
                        on:input=move |event| text.set(event_target_value(&event))
                        on:keydown=move |event: web_sys::KeyboardEvent| {
                            if event.key() == "Enter" {
                                confirm();
                            } else if event.key() == "Escape" {
                                state.editing.prompt.set(None);
                            }
                        }
                    />
                </div>
            </div>
        </Show>
    }
}

fn join(dir: &str, name: &str) -> String {
    let separator = if dir.contains('\\') { '\\' } else { '/' };
    format!("{dir}{separator}{name}")
}

fn parent_of(path: &str) -> String {
    match path.rfind(['\\', '/']) {
        Some(index) => path[..index].to_string(),
        None => String::new(),
    }
}
