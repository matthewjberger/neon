//! The terminal panel: a scrollback of task and shell output plus a command
//! input. Typed commands run through the platform shell, cargo commands stream
//! here too, and a running task can be cancelled.

use leptos::html;
use leptos::prelude::*;

use crate::state::EditorState;

#[component]
pub fn TaskPanel(state: EditorState) -> impl IntoView {
    let input = RwSignal::new(String::new());
    let input_ref = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        if state.task_open.get()
            && let Some(element) = input_ref.get()
        {
            let _ = element.focus();
        }
    });
    let submit = move || {
        let command = input.get_untracked();
        let command = command.trim();
        if !command.is_empty() {
            crate::tasks::run_shell(state, command);
            input.set(String::new());
        }
    };
    view! {
        <Show when=move || state.task_open.get() fallback=|| ()>
            <div class="task-panel">
                <div class="task-header">
                    <span>
                        {move || if state.task_running.get() { "Terminal (running)" } else { "Terminal" }}
                    </span>
                    <span class="task-actions">
                        <Show when=move || state.task_running.get() fallback=|| ()>
                            <button
                                class="icon-button"
                                on:click=move |_| crate::tasks::cancel(state)
                            >
                                "Cancel"
                            </button>
                        </Show>
                        <button
                            class="icon-button"
                            on:click=move |_| crate::tasks::clear_output(state)
                        >
                            "Clear"
                        </button>
                        <button class="icon-button" on:click=move |_| state.task_open.set(false)>
                            "x"
                        </button>
                    </span>
                </div>
                <div class="task-body">
                    <For
                        each=move || { state.task_output.get().into_iter().enumerate().collect::<Vec<_>>() }
                        key=|(index, _)| *index
                        children=move |(_, line)| view! { <div class="task-line">{line}</div> }
                    />
                </div>
                <input
                    class="task-input"
                    node_ref=input_ref
                    placeholder="Run a command"
                    prop:value=move || input.get()
                    on:input=move |event| input.set(event_target_value(&event))
                    on:keydown=move |event| {
                        if event.key() == "Enter" {
                            submit();
                        }
                    }
                />
            </div>
        </Show>
    }
}
