//! The AI edit review: when an agent proposes a buffer edit through
//! `propose_edit`, it is staged rather than applied, and shown here as a
//! line-level diff. Accept applies the proposed text to the plugin and re-syncs
//! the scene; reject discards it.

use leptos::prelude::*;
use protocol::{LineChange, diff_lines};

use crate::bridge::{self, Bridge};
use crate::state::EditorState;

#[component]
pub fn DiffReview(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    state: EditorState,
) -> impl IntoView {
    let accept = move |_| {
        if let Some((buffer, text)) = state.editing.proposal.get_untracked() {
            state.plugins.update(|plugins| {
                if let Some(plugin) = plugins.iter_mut().find(|plugin| plugin.id == buffer) {
                    plugin.source = text.clone();
                }
            });
            crate::plugins::save(&state.plugins.get_untracked());
            if let Some(connected) = bridge.get_value() {
                bridge::sync_plugins(&connected, state);
            }
            state.editing.proposal.set(None);
        }
    };
    let reject = move |_| state.editing.proposal.set(None);
    view! {
        <Show when=move || state.editing.proposal.get().is_some() fallback=|| ()>
            <div class="diff-review-overlay">
                <div class="diff-review">
                    <div class="diff-review-header">
                        <span>"Proposed edit"</span>
                        <button class="tool-button" on:click=accept>
                            "Accept"
                        </button>
                        <button class="tool-button" on:click=reject>
                            "Reject"
                        </button>
                    </div>
                    <div class="diff-review-body">
                        {move || {
                            let Some((buffer, proposed)) = state.editing.proposal.get() else {
                                return ().into_any();
                            };
                            let current = state
                                .plugins
                                .with(|plugins| {
                                    plugins
                                        .iter()
                                        .find(|plugin| plugin.id == buffer)
                                        .map(|plugin| plugin.source.clone())
                                        .unwrap_or_default()
                                });
                            diff_lines(&current, &proposed)
                                .into_iter()
                                .map(|line| {
                                    let (class, sign) = match line.change {
                                        LineChange::Equal => ("diff-line diff-equal", " "),
                                        LineChange::Insert => ("diff-line diff-insert", "+"),
                                        LineChange::Delete => ("diff-line diff-delete", "-"),
                                    };
                                    view! {
                                        <div class=class>
                                            <span class="diff-sign">{sign}</span>
                                            <span class="diff-text">{line.text}</span>
                                        </div>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }}
                    </div>
                </div>
            </div>
        </Show>
    }
}
