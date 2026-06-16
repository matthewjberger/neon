//! The tour card: a small, non-blocking panel that shows the current step and
//! its instruction. It floats over the editor so you can perform each action
//! while it is open, and advances itself when you do (see `crate::tour`).

use leptos::prelude::*;

use crate::state::EditorState;
use crate::tour::{STEPS, close, next};

#[component]
pub fn TourView(state: EditorState) -> impl IntoView {
    view! {
        <Show when=move || state.tour.get().is_some() fallback=|| ()>
            <div class="tour-card">
                {move || {
                    let index = state.tour.get().unwrap_or(0);
                    let step = STEPS.get(index);
                    let title = step.map(|step| step.title).unwrap_or_default();
                    let body = step.map(|step| step.body).unwrap_or_default();
                    let last = index + 1 >= STEPS.len();
                    view! {
                        <div class="tour-header">
                            <span class="tour-title">{title}</span>
                            <span class="tour-progress">
                                {format!("{} / {}", index + 1, STEPS.len())}
                            </span>
                        </div>
                        <div class="tour-body">{body}</div>
                        <div class="tour-actions">
                            <button class="tool-button" on:click=move |_| close(state)>
                                "Skip"
                            </button>
                            <button class="tool-button" on:click=move |_| next(state)>
                                {if last { "Finish" } else { "Next" }}
                            </button>
                        </div>
                    }
                }}
            </div>
        </Show>
    }
}
