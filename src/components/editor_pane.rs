//! The code editor for the active plugin: a native textarea for editing, a
//! highlight `<pre>` layer behind it sharing the same box, and a diagnostics
//! strip below. Edits update the plugin source, persist, and after a short pause
//! sync the scene and ask the language worker to compile-check.

use std::collections::HashSet;

use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::bridge::{self, Bridge};
use crate::editor_plugins;
use crate::highlight::highlight;
use crate::lang::{self, Lang};
use crate::state::{EditorState, PluginKind};

const APPLY_DELAY_MS: i32 = 350;

#[component]
pub fn EditorPane(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    lang: StoredValue<Option<Lang>, LocalStorage>,
    state: EditorState,
) -> impl IntoView {
    let layer = NodeRef::<html::Pre>::new();
    let textarea = NodeRef::<html::Textarea>::new();
    let debounce = StoredValue::new(None::<i32>);
    let request_id = StoredValue::new(0_u32);

    let command_set = Memo::new(move |_| {
        state
            .commands
            .get()
            .into_iter()
            .map(|command| command.method)
            .collect::<HashSet<String>>()
    });

    let on_input = move |event: web_sys::Event| {
        let value = event_target_value(&event);
        let Some(id) = state.active.get_untracked() else {
            return;
        };
        state.active_signal().update(|plugins| {
            if let Some(plugin) = plugins.iter_mut().find(|plugin| plugin.id == id) {
                plugin.source = value.clone();
            }
        });
        commit(bridge, lang, state, debounce, request_id);
    };

    let on_keydown = move |event: web_sys::KeyboardEvent| {
        if !editor_plugins::any_enabled(state) {
            return;
        }
        let Some(textarea) = textarea.get() else {
            return;
        };
        let outcome = editor_plugins::handle_key(
            state,
            &textarea,
            &event.key(),
            event.ctrl_key(),
            event.shift_key(),
            event.alt_key(),
        );
        if outcome.consumed {
            event.prevent_default();
        }
        if outcome.changed {
            commit(bridge, lang, state, debounce, request_id);
        }
    };

    view! {
        <div class="editor-pane">
            <Show
                when=move || state.active.get().is_some()
                fallback=|| view! { <div class="editor-empty">"Select or create a plugin to edit"</div> }
            >
                <div class="editor-wrap">
                    <pre class="editor-highlight" node_ref=layer aria-hidden="true">
                        {move || {
                            let source = state.active_source();
                            let set = command_set.get();
                            highlight(&source, &set)
                                .into_iter()
                                .map(|(class, run)| view! { <span class=class>{run}</span> })
                                .collect_view()
                        }}
                    </pre>
                    <textarea
                        class="editor-textarea"
                        spellcheck="false"
                        node_ref=textarea
                        prop:value=move || state.active_source()
                        on:input=on_input
                        on:keydown=on_keydown
                        on:scroll=move |event| {
                            if let Some(layer) = layer.get()
                                && let Some(target) = event.target()
                                && let Ok(element) = target.dyn_into::<web_sys::HtmlElement>()
                            {
                                layer.set_scroll_top(element.scroll_top());
                                layer.set_scroll_left(element.scroll_left());
                            }
                        }
                    />
                </div>
                <div class="diagnostics">
                    <For
                        each=move || { state.diagnostics.get().into_iter().enumerate().collect::<Vec<_>>() }
                        key=|(index, _)| *index
                        children=move |(_, diag)| {
                            view! {
                                <div class="diagnostic">
                                    <span class="diag-pos">
                                        {format!("{}:{}", diag.line, diag.column)}
                                    </span>
                                    {diag.message}
                                </div>
                            }
                        }
                    />
                </div>
            </Show>
        </div>
    }
}

/// Persists the active buffer and, for a scene plugin, schedules the worker sync
/// and compile-check. Editor plugins persist through the app effect and run live,
/// so they need no sync.
fn commit(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    lang: StoredValue<Option<Lang>, LocalStorage>,
    state: EditorState,
    debounce: StoredValue<Option<i32>>,
    request_id: StoredValue<u32>,
) {
    if state.active_kind.get_untracked() == PluginKind::Scene {
        schedule_apply(bridge, lang, state, debounce, request_id);
    }
}

fn schedule_apply(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    lang: StoredValue<Option<Lang>, LocalStorage>,
    state: EditorState,
    debounce: StoredValue<Option<i32>>,
    request_id: StoredValue<u32>,
) {
    let Some(window) = web_sys::window() else {
        return;
    };
    if let Some(handle) = debounce.get_value() {
        window.clear_timeout_with_handle(handle);
    }
    let callback = Closure::once_into_js(move || {
        if let Some(bridge) = bridge.get_value() {
            bridge::sync_plugins(&bridge, state);
        }
        let id = request_id.get_value().wrapping_add(1);
        request_id.set_value(id);
        if let Some(lang) = lang.get_value() {
            lang::check(&lang, id, state.active_source());
        }
    });
    let handle = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.unchecked_ref(),
            APPLY_DELAY_MS,
        )
        .unwrap_or(0);
    debounce.set_value(Some(handle));
}
