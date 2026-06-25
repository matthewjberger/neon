//! The reference overlay: every api command and standard-library helper, from
//! the manifest the worker sent, searchable. The same source the highlighter and
//! the language worker validate against, so it never drifts.

use leptos::prelude::*;

use crate::state::EditorState;

#[component]
pub fn Reference(state: EditorState) -> impl IntoView {
    let filter = RwSignal::new(String::new());

    let commands = move || {
        let needle = filter.get().to_lowercase();
        state
            .commands
            .get()
            .into_iter()
            .filter(|command| needle.is_empty() || command.method.contains(&needle))
            .collect::<Vec<_>>()
    };

    let helpers = move || {
        let needle = filter.get().to_lowercase();
        state
            .stdlib
            .get()
            .into_iter()
            .flat_map(|module| module.helpers)
            .filter(|helper| needle.is_empty() || helper.name.contains(&needle))
            .collect::<Vec<_>>()
    };

    view! {
        <Show when=move || state.panels.reference.get() fallback=|| ()>
            <div class="reference-overlay" on:click=move |_| state.panels.reference.set(false)>
                <div class="reference-panel" on:click=move |event| event.stop_propagation()>
                    <input
                        class="reference-search"
                        placeholder="filter commands and helpers"
                        prop:value=move || filter.get()
                        on:input=move |event| filter.set(event_target_value(&event))
                    />
                    <div class="reference-scroll">
                        <div class="reference-section">"Standard library"</div>
                        <For
                            each=helpers
                            key=|helper| helper.name.clone()
                            children=move |helper| {
                                view! {
                                    <div class="reference-row">
                                        <span class="tok-command">{helper.signature.clone()}</span>
                                        <span class="reference-desc">{helper.description.clone()}</span>
                                    </div>
                                }
                            }
                        />
                        <div class="reference-section">"Commands"</div>
                        <For
                            each=commands
                            key=|command| command.method.clone()
                            children=move |command| {
                                let params = command
                                    .fields
                                    .iter()
                                    .map(|field| field.name.clone())
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                view! {
                                    <div class="reference-row">
                                        <span class="tok-command">
                                            {format!("{}({params})", command.method)}
                                        </span>
                                        <span class="reference-desc">{command.description.clone()}</span>
                                    </div>
                                }
                            }
                        />
                    </div>
                </div>
            </div>
        </Show>
    }
}
