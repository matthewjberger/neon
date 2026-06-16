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
use crate::state::{EditorState, PluginKind, kind_readonly, language_for_path};

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
    let buffer = move || pane().and_then(|pane| pane.buffer().cloned());
    let active_id = move || buffer().and_then(|buffer| buffer.id);
    let active_kind = move || {
        buffer()
            .map(|buffer| buffer.kind)
            .unwrap_or(PluginKind::Scene)
    };
    let source = move || state.buffer_source(active_kind(), &active_id());
    let readonly = move || kind_readonly(active_kind());
    let flex = move || pane().map(|pane| pane.flex as f64).unwrap_or(1.0);
    let focused = move || state.pane_count() > 1 && state.focused_key.get() == pane_key;

    let current = move || {
        state.panes.with_untracked(|panes| {
            panes
                .iter()
                .find(|pane| pane.key == pane_key)
                .and_then(|pane| pane.buffer().cloned())
                .map(|buffer| (buffer.id, buffer.kind))
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

    let on_focus = move |_| {
        state.focused_key.set(pane_key);
        if let Some(element) = textarea.get() {
            crate::components::find::set_active(element);
        }
    };

    let on_input = move |event: web_sys::Event| {
        let (id, kind) = current();
        if kind_readonly(kind) {
            return;
        }
        let value = event_target_value(&event);
        if id.is_none() {
            return;
        }
        state.set_buffer_text(kind, &id, value);
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
            <div class="tab-bar">
                {move || {
                    let current_pane = pane();
                    let active = current_pane.as_ref().map(|pane| pane.active).unwrap_or(0);
                    current_pane
                        .map(|pane| pane.tabs)
                        .unwrap_or_default()
                        .into_iter()
                        .enumerate()
                        .map(|(index, tab)| {
                            let name = state.buffer_name(tab.kind, &tab.id);
                            let dirty = state.is_dirty(tab.kind, &tab.id);
                            view! {
                                <div
                                    class="tab"
                                    class:active=index == active
                                    on:click=move |_| state.focus_tab(pane_key, index)
                                >
                                    <span class="tab-name">{name}</span>
                                    <Show when=move || dirty fallback=|| ()>
                                        <span class="tab-dirty">"\u{2022}"</span>
                                    </Show>
                                    <button
                                        class="tab-close"
                                        on:click=move |event: web_sys::MouseEvent| {
                                            event.stop_propagation();
                                            state.close_tab(pane_key, index);
                                        }
                                    >
                                        "\u{00d7}"
                                    </button>
                                </div>
                            }
                        })
                        .collect_view()
                }}
            </div>
            <Show
                when=move || active_id().is_some()
                fallback=|| view! { <div class="editor-empty">"Open a buffer to edit"</div> }
            >
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
                            let language = match active_kind() {
                                PluginKind::File => active_id()
                                    .as_deref()
                                    .map(language_for_path)
                                    .unwrap_or("plaintext"),
                                _ => "rhai",
                            };
                            highlight(&text, language, &set)
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
    let buffer = state.panes.with_untracked(|panes| {
        panes
            .iter()
            .find(|pane| pane.key == pane_key)
            .and_then(|pane| pane.buffer().cloned())
    });
    let Some(buffer) = buffer else {
        return;
    };
    match buffer.kind {
        PluginKind::Scene => {
            schedule_apply(bridge, lang, state, pane_key, debounce, request_id);
        }
        PluginKind::File => {
            if let Some(path) = buffer.id {
                crate::lsp::did_change(state, &path);
            }
        }
        _ => {}
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
                    .and_then(|pane| pane.buffer().cloned())
                    .map(|buffer| (buffer.id, buffer.kind))
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
