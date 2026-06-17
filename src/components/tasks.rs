//! The task output panel: the streamed output of cargo and other tasks, with a
//! running indicator and a cancel button.

use leptos::prelude::*;

use crate::state::EditorState;

#[component]
pub fn TaskPanel(state: EditorState) -> impl IntoView {
    view! {
        <Show when=move || state.task_open.get() fallback=|| ()>
            <div class="task-panel">
                <div class="task-header">
                    <span>
                        {move || if state.task_running.get() { "Task (running)" } else { "Task" }}
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
            </div>
        </Show>
    }
}
