use leptos::prelude::*;

use crate::state::EditorState;

/// The top progress bar plus the startup card. The thin indeterminate bar at the
/// top of the window shows while the renderer starts and whenever the worker is
/// rebuilding the scene. The centered card stays up until the renderer is ready.
#[component]
pub fn Loader(state: EditorState) -> impl IntoView {
    let bar_active = move || !state.ready.get() || state.busy.get();

    view! {
        <Show when=bar_active fallback=|| ()>
            <div class="loading-track">
                <div class="loading-bar"></div>
            </div>
        </Show>
        <Show when=move || !state.ready.get() fallback=|| ()>
            <div class="loader-overlay">
                <div class="loader-card">
                    <span class="spinner"></span>
                    "Starting the renderer..."
                </div>
            </div>
        </Show>
    }
}
