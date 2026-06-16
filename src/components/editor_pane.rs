//! The code editor for one pane: a native textarea, a highlight `<pre>` layer
//! behind it, a line-number gutter, and a diagnostics strip. The pane renders the
//! buffer named by its entry in `state.panes`, keyed by `pane_key`, so every
//! split pane edits independently. Built-ins are read-only.

use std::collections::HashSet;

use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::bridge::{self, Bridge};
use crate::editor_plugins;
use crate::highlight::highlight;
use crate::lang::{self, Lang};
use crate::state::{EditorState, PluginKind, kind_readonly};

const APPLY_DELAY_MS: i32 = 350;

#[component]
pub fn EditorPane(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    lang: StoredValue<Option<Lang>, LocalStorage>,
    state: EditorState,
    /// The stable key of the pane this component renders.
    pane_key: usize,
) -> impl IntoView {
    let layer = NodeRef::<html::Pre>::new();
    let gutter = NodeRef::<html::Div>::new();
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

    let pane = move || {
        state
            .panes
            .with(|panes| panes.iter().find(|pane| pane.key == pane_key).cloned())
    };
    let active_id = move || pane().and_then(|pane| pane.active);
    let active_kind = move || pane().map(|pane| pane.kind).unwrap_or(PluginKind::Scene);
    let source = move || state.buffer_source(active_kind(), &active_id());
    let readonly = move || kind_readonly(active_kind());
    let flex = move || pane().map(|pane| pane.flex as f64).unwrap_or(1.0);
    let focused = move || state.pane_count() > 1 && state.focused_key.get() == pane_key;

    let current = move || {
        state.panes.with_untracked(|panes| {
            panes
                .iter()
                .find(|pane| pane.key == pane_key)
                .map(|pane| (pane.active.clone(), pane.kind))
                .unwrap_or((None, PluginKind::Scene))
        })
    };

    Effect::new(move |_| {
        if state.focused_key.get() == pane_key
            && let Some(element) = textarea.get()
        {
            let _ = element.focus();
        }
    });

    let on_focus = move |_| state.focused_key.set(pane_key);

    let on_input = move |event: web_sys::Event| {
        let (id, kind) = current();
        if kind_readonly(kind) {
            return;
        }
        let value = event_target_value(&event);
        let Some(id) = id else {
            return;
        };
        state.editable_set(kind).update(|plugins| {
            if let Some(plugin) = plugins.iter_mut().find(|plugin| plugin.id == id) {
                plugin.source = value.clone();
            }
        });
        commit(bridge, lang, state, pane_key, debounce, request_id);
    };

    let on_keydown = move |event: web_sys::KeyboardEvent| {
        let (id, kind) = current();
        if kind_readonly(kind) {
            return;
        }
        if event.key() == "Tab" {
            event.prevent_default();
            if let Some(element) = textarea.get() {
                editor_plugins::insert_text(state, id, kind, &element, "    ");
                commit(bridge, lang, state, pane_key, debounce, request_id);
            }
            return;
        }
        if !editor_plugins::any_enabled(state) {
            return;
        }
        let Some(element) = textarea.get() else {
            return;
        };
        let outcome = editor_plugins::handle_key(
            state,
            id,
            kind,
            &element,
            &editor_plugins::KeyEvent {
                key: event.key(),
                ctrl: event.ctrl_key(),
                shift: event.shift_key(),
                alt: event.alt_key(),
            },
        );
        if outcome.consumed {
            event.prevent_default();
        }
        if outcome.changed {
            commit(bridge, lang, state, pane_key, debounce, request_id);
        }
    };

    view! {
        <div
            class="editor-pane"
            class:focused=focused
            style:flex-grow=move || flex().to_string()
        >
            <Show
                when=move || active_id().is_some()
                fallback=|| view! { <div class="editor-empty">"Open a plugin to edit"</div> }
            >
                <div class="editor-header">
                    <span class="editor-filename">
                        {move || state.buffer_name(active_kind(), &active_id())}
                    </span>
                    <Show when=readonly fallback=|| ()>
                        <span class="editor-lock">"read-only built-in"</span>
                    </Show>
                </div>
                <div class="editor-wrap">
                    <div class="editor-gutter" node_ref=gutter>
                        {move || {
                            let count = source().split('\n').count().max(1);
                            (1..=count).map(|number| view! { <div>{number}</div> }).collect_view()
                        }}
                    </div>
                    <pre class="editor-highlight" node_ref=layer aria-hidden="true">
                        {move || {
                            let text = source();
                            let set = command_set.get();
                            highlight(&text, &set)
                                .into_iter()
                                .map(|(class, run)| view! { <span class=class>{run}</span> })
                                .collect_view()
                        }}
                    </pre>
                    <textarea
                        class="editor-textarea"
                        spellcheck="false"
                        node_ref=textarea
                        prop:readonly=readonly
                        prop:value=source
                        on:focus=on_focus
                        on:input=on_input
                        on:keydown=on_keydown
                        on:scroll=move |event| {
                            if let Some(target) = event.target()
                                && let Ok(element) = target.dyn_into::<web_sys::HtmlElement>()
                            {
                                if let Some(layer) = layer.get() {
                                    layer.set_scroll_top(element.scroll_top());
                                    layer.set_scroll_left(element.scroll_left());
                                }
                                if let Some(gutter) = gutter.get() {
                                    gutter.set_scroll_top(element.scroll_top());
                                }
                            }
                        }
                    />
                </div>
                <Show when=focused fallback=|| ()>
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
            </Show>
        </div>
    }
}

/// Persists the buffer and, for a scene plugin, schedules the worker sync and
/// compile-check.
fn commit(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    lang: StoredValue<Option<Lang>, LocalStorage>,
    state: EditorState,
    pane_key: usize,
    debounce: StoredValue<Option<i32>>,
    request_id: StoredValue<u32>,
) {
    let kind = state
        .panes
        .with_untracked(|panes| {
            panes
                .iter()
                .find(|pane| pane.key == pane_key)
                .map(|pane| pane.kind)
        })
        .unwrap_or(PluginKind::Scene);
    if kind == PluginKind::Scene {
        schedule_apply(bridge, lang, state, pane_key, debounce, request_id);
    }
}

fn schedule_apply(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    lang: StoredValue<Option<Lang>, LocalStorage>,
    state: EditorState,
    pane_key: usize,
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
            let (active, kind) = state.panes.with_untracked(|panes| {
                panes
                    .iter()
                    .find(|pane| pane.key == pane_key)
                    .map(|pane| (pane.active.clone(), pane.kind))
                    .unwrap_or((None, PluginKind::Scene))
            });
            lang::check(&lang, id, state.buffer_source(kind, &active));
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
