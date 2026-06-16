//! The activity bar: the narrow icon rail that switches the sidebar view, the
//! VSCode-familiar left edge.

use leptos::prelude::*;

use crate::state::{EditorState, SidebarView};

#[component]
pub fn ActivityBar(state: EditorState) -> impl IntoView {
    view! {
        <div class="activity-bar">
            <button
                class="activity-icon"
                class:active=move || state.sidebar_view.get() == SidebarView::Installed
                title="Installed plugins"
                on:click=move |_| state.sidebar_view.set(SidebarView::Installed)
            >
                "\u{2263}"
            </button>
            <button
                class="activity-icon"
                class:active=move || state.sidebar_view.get() == SidebarView::Extensions
                title="Manage plugins"
                on:click=move |_| state.sidebar_view.set(SidebarView::Extensions)
            >
                "\u{229e}"
            </button>
        </div>
    }
}
