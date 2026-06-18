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
    let hover_timer = StoredValue::new(None::<i32>);
    let completion_timer = StoredValue::new(None::<i32>);

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

    Effect::new(move |_| {
        if state.focused_key.get() == pane_key
            && let Some(element) = textarea.get()
        {
            let _ = element.focus();
        }
    });

    Effect::new(move |_| {
        let goto = state.goto.get();
        let current = buffer();
        let Some((path, line)) = goto else {
            return;
        };
        let Some(current) = current else {
            return;
        };
        if current.kind != PluginKind::File || current.id.as_deref() != Some(path.as_str()) {
            return;
        }
        let Some(element) = textarea.get() else {
            return;
        };
        state.goto.set(None);
        let callback = Closure::once_into_js(move || {
            let value = element.value();
            let mut offset = 0_u32;
            for (number, segment) in value.split_inclusive('\n').enumerate() {
                if number as u32 + 1 >= line {
                    break;
                }
                offset += segment.encode_utf16().count() as u32;
            }
            let _ = element.focus();
            let _ = element.set_selection_range(offset, offset);
        });
        if let Some(window) = web_sys::window() {
            let _ = window
                .set_timeout_with_callback_and_timeout_and_arguments_0(callback.unchecked_ref(), 0);
        }
    });

    let on_focus = move |_| {
        state.focused_key.set(pane_key);
        if let Some(element) = textarea.get() {
            crate::components::find::set_active(element);
        }
    };

    let on_mousemove = move |event: web_sys::MouseEvent| {
        let x = event.client_x() as f64;
        let y = event.client_y() as f64;
        debounce_timer(hover_timer, 400, move || {
            crate::lsp::request_hover_at(state, x, y)
        });
    };

    let on_mouseleave = move |_| state.hover.set(None);

    let on_input = move |event: web_sys::Event| {
        let (id, kind) = current_buffer(state, pane_key);
        if kind_readonly(kind) {
            return;
        }
        let value = event_target_value(&event);
        if id.is_none() {
            return;
        }
        state.set_buffer_text(kind, &id, value);
        commit(bridge, lang, state, pane_key, debounce, request_id);
        if kind == PluginKind::File {
            debounce_timer(completion_timer, 150, move || {
                crate::lsp::request_completion(state)
            });
            if let Some(element) = textarea.get() {
                let caret = element.selection_start().ok().flatten().unwrap_or(0);
                match char_before(&element.value(), caret) {
                    Some('(') | Some(',') => crate::lsp::request_signature_help(state),
                    Some(')') => state.hover.set(None),
                    _ => {}
                }
            }
        }
        if matches!(kind, PluginKind::Scene | PluginKind::Editor) {
            debounce_timer(completion_timer, 150, move || {
                crate::complete::rhai_complete(state)
            });
        }
    };

    let on_keydown = move |event: web_sys::KeyboardEvent| {
        handle_keydown(KeyContext {
            event,
            state,
            bridge,
            lang,
            pane_key,
            textarea,
            debounce,
            request_id,
        });
    };

    view! {
        <div
            class="editor-pane"
            class:focused=focused
            style:flex-grow=move || flex().to_string()
        >
            <TabBar state pane_key />
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
                            state.editor_scroll.get();
                            let language = match active_kind() {
                                PluginKind::File => active_id()
                                    .as_deref()
                                    .map(language_for_path)
                                    .unwrap_or("plaintext"),
                                _ => "rhai",
                            };
                            let segments: Vec<&str> = text.split_inclusive('\n').collect();
                            let (first, last) = window_range(textarea.get(), segments.len());
                            let before: String = segments[..first].concat();
                            let window: String = segments[first..last].concat();
                            let after: String = segments[last..].concat();
                            let mut views = Vec::new();
                            if !before.is_empty() {
                                views.push(view! { <span>{before}</span> }.into_any());
                            }
                            for (class, run) in highlight(&window, language, &set) {
                                views.push(view! { <span class=class>{run}</span> }.into_any());
                            }
                            if !after.is_empty() {
                                views.push(view! { <span>{after}</span> }.into_any());
                            }
                            views.into_iter().collect_view()
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
                        on:mousemove=on_mousemove
                        on:mouseleave=on_mouseleave
                        on:contextmenu=move |event: web_sys::MouseEvent| {
                            event.prevent_default();
                            event.stop_propagation();
                            crate::components::context_menu::open(
                                state,
                                event.client_x() as f64,
                                event.client_y() as f64,
                                crate::components::context_menu::editor_menu(),
                            );
                        }
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
                            state.editor_scroll.update(|tick| *tick = tick.wrapping_add(1));
                        }
                        on:mousedown=move |_| crate::multicursor::clear(state)
                    />
                </div>
                <DiagnosticStrip state pane_key />
            </Show>
        </div>
    }
}

/// The tab strip for one pane: a draggable, closable tab per open buffer.
#[component]
fn TabBar(state: EditorState, pane_key: usize) -> impl IntoView {
    let dragging = StoredValue::new(None::<usize>);
    let pane = move || {
        state
            .panes
            .with(|panes| panes.iter().find(|pane| pane.key == pane_key).cloned())
    };
    view! {
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
                                draggable="true"
                                on:click=move |_| state.focus_tab(pane_key, index)
                                on:dragstart=move |_| dragging.set_value(Some(index))
                                on:dragover=move |event: web_sys::DragEvent| {
                                    if dragging.get_value().is_some() {
                                        event.prevent_default();
                                    }
                                }
                                on:drop=move |event: web_sys::DragEvent| {
                                    event.prevent_default();
                                    if let Some(from) = dragging.get_value() {
                                        state.move_tab(pane_key, from, index);
                                    }
                                    dragging.set_value(None);
                                }
                                on:dragend=move |_| dragging.set_value(None)
                                on:mousedown=move |event: web_sys::MouseEvent| {
                                    if event.button() == 1 {
                                        event.prevent_default();
                                        state.close_tab(pane_key, index);
                                    }
                                }
                                on:contextmenu=move |event: web_sys::MouseEvent| {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    state.focus_tab(pane_key, index);
                                    crate::components::context_menu::open(
                                        state,
                                        event.client_x() as f64,
                                        event.client_y() as f64,
                                        crate::components::context_menu::tab_menu(),
                                    );
                                }
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
    }
}

/// The diagnostics strip under the editor, shown only for the focused pane so a
/// split does not stack a strip per pane.
#[component]
fn DiagnosticStrip(state: EditorState, pane_key: usize) -> impl IntoView {
    view! {
        <Show when=move || state.focused_key.get() == pane_key fallback=|| ()>
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
    }
}

/// The handles a keystroke needs to read and edit the focused buffer, bundled so
/// the router is a free function over plain data rather than a closure capturing
/// the component scope.
struct KeyContext {
    event: web_sys::KeyboardEvent,
    state: EditorState,
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    lang: StoredValue<Option<Lang>, LocalStorage>,
    pane_key: usize,
    textarea: NodeRef<html::Textarea>,
    debounce: StoredValue<Option<i32>>,
    request_id: StoredValue<u32>,
}

/// Routes one keydown: the completion popup first, then multi-cursor, then the
/// built-in newline and tab handling, then the editor plugins.
fn handle_keydown(ctx: KeyContext) {
    let KeyContext {
        event,
        state,
        bridge,
        lang,
        pane_key,
        textarea,
        debounce,
        request_id,
    } = ctx;
    if state.jump.get_untracked().is_some() {
        return;
    }
    if state.completion.get_untracked().is_some() {
        let len = state
            .completion
            .with_untracked(|menu| menu.as_ref().map(|menu| menu.items.len()).unwrap_or(0))
            .max(1);
        match event.key().as_str() {
            "ArrowDown" => {
                event.prevent_default();
                state
                    .completion_index
                    .update(|index| *index = (*index + 1) % len);
                return;
            }
            "ArrowUp" => {
                event.prevent_default();
                state
                    .completion_index
                    .update(|index| *index = (*index + len - 1) % len);
                return;
            }
            "Enter" | "Tab" => {
                event.prevent_default();
                crate::lsp::accept_completion(state, state.completion_index.get_untracked());
                return;
            }
            "Escape" => {
                event.prevent_default();
                state.completion.set(None);
                return;
            }
            _ => {}
        }
    }
    let (id, kind) = current_buffer(state, pane_key);
    if kind_readonly(kind) {
        return;
    }
    if crate::multicursor::active(state) {
        match event.key().as_str() {
            "Escape" => {
                event.prevent_default();
                crate::multicursor::clear(state);
                return;
            }
            "Backspace" => {
                event.prevent_default();
                crate::multicursor::delete_back(state);
                return;
            }
            "Delete" => {
                event.prevent_default();
                crate::multicursor::delete_forward(state);
                return;
            }
            "Enter" => {
                event.prevent_default();
                crate::multicursor::insert(state, "\n");
                return;
            }
            "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | "Home" | "End" | "PageUp"
            | "PageDown" => {
                if !event.ctrl_key() && !event.alt_key() && !event.meta_key() {
                    crate::multicursor::clear(state);
                }
            }
            key => {
                if !event.ctrl_key()
                    && !event.alt_key()
                    && !event.meta_key()
                    && key.chars().count() == 1
                {
                    event.prevent_default();
                    crate::multicursor::insert(state, key);
                    return;
                }
            }
        }
    }
    if event.key() == "Enter" && inserts_newline(state) {
        if let Some(element) = textarea.get() {
            event.prevent_default();
            let caret = element.selection_start().ok().flatten().unwrap_or(0);
            let text = newline_indent(&element.value(), caret);
            editor_plugins::insert_text(state, id, kind, &element, &text);
            commit(bridge, lang, state, pane_key, debounce, request_id);
        }
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
}

/// The focused buffer's id and kind for a pane, or the scene default.
fn current_buffer(state: EditorState, pane_key: usize) -> (Option<String>, PluginKind) {
    state.panes.with_untracked(|panes| {
        panes
            .iter()
            .find(|pane| pane.key == pane_key)
            .and_then(|pane| pane.buffer().cloned())
            .map(|buffer| (buffer.id, buffer.kind))
            .unwrap_or((None, PluginKind::Scene))
    })
}

/// Resets a timer to fire `action` once after `delay` ms, replacing any pending
/// fire. The single scheduler the hover and completion debounces share.
fn debounce_timer(timer: StoredValue<Option<i32>>, delay: i32, action: impl FnOnce() + 'static) {
    let Some(window) = web_sys::window() else {
        return;
    };
    if let Some(handle) = timer.get_value() {
        window.clear_timeout_with_handle(handle);
    }
    let callback = Closure::once_into_js(action);
    let handle = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(callback.unchecked_ref(), delay)
        .unwrap_or(0);
    timer.set_value(Some(handle));
}

/// The line range to highlight: the lines scrolled into view plus a buffer, so
/// the highlighter scans the window instead of the whole buffer. Lines outside
/// it render as plain text, keeping the full text and every line position exact.
fn window_range(element: Option<web_sys::HtmlTextAreaElement>, total: usize) -> (usize, usize) {
    const BUFFER: usize = 40;
    let total = total.max(1);
    let Some(element) = element else {
        return (0, total.min(400));
    };
    let line_height = crate::caret::line_height(&element).max(1.0);
    let scroll = element.scroll_top() as f64;
    let view = (element.client_height() as f64).max(line_height);
    let first = ((scroll / line_height) as usize).saturating_sub(BUFFER);
    let count = (view / line_height) as usize + BUFFER * 2 + 1;
    let last = (first + count).min(total);
    (first, last)
}

/// Whether pressing Enter should insert a newline here: in insert mode, or when
/// no modal layer is active (a modal layer consumes Enter in normal mode).
fn inserts_newline(state: EditorState) -> bool {
    if state.editor_mode.get_untracked() == "insert" {
        return true;
    }
    !state
        .editor_plugins
        .get_untracked()
        .iter()
        .any(|plugin| plugin.enabled && crate::plugins::is_modal(&plugin.id))
}

/// The text to insert for an auto-indented newline: a line break, the current
/// line's leading whitespace, and one more level when the line opens a block.
fn newline_indent(value: &str, caret: u32) -> String {
    let units: Vec<u16> = value.encode_utf16().collect();
    let end = (caret as usize).min(units.len());
    let before = String::from_utf16_lossy(&units[..end]);
    let line_start = before.rfind('\n').map(|index| index + 1).unwrap_or(0);
    let line = &before[line_start..];
    let indent: String = line
        .chars()
        .take_while(|character| *character == ' ' || *character == '\t')
        .collect();
    let mut result = String::from("\n");
    result.push_str(&indent);
    if line.trim_end().ends_with('{') {
        result.push_str("    ");
    }
    result
}

/// The character immediately before a UTF-16 caret offset, resolved by walking
/// code points so it stays correct across multibyte characters.
fn char_before(value: &str, caret: u32) -> Option<char> {
    let mut units = 0;
    let mut previous = None;
    for character in value.chars() {
        let next = units + character.len_utf16() as u32;
        if next > caret {
            break;
        }
        previous = Some(character);
        units = next;
    }
    previous
}

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
