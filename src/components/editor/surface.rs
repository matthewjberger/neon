//! The custom editing surface: a rope-backed [`Document`] rendered as DOM, with
//! native multi-cursor. It is the opt-in alternative to the textarea (toggled by
//! `state.editing.surface`). A hidden textarea captures keystrokes, IME, and
//! clipboard; the text is rendered as DOM, and the caret and selection overlays
//! are positioned by measuring the rendered glyphs with a `Range`, so inline
//! decorations stay aligned. Edits persist through `set_buffer_text`, so
//! highlighting and saving keep working. The modal plugin layer runs against the
//! document through `handle_key_document`, including macros, the dot-repeat, and
//! the view motions, so the surface is a full modal editor.

use std::collections::HashSet;

use document::{Document, Selection};
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

use crate::highlight::highlight;
use crate::state::{EditorState, PluginKind, language_for_path};

use super::current_buffer;

#[component]
pub fn CodeSurface(state: EditorState, pane_key: usize) -> impl IntoView {
    let input = NodeRef::<html::Textarea>::new();
    let content = NodeRef::<html::Div>::new();
    let text_node = NodeRef::<html::Div>::new();
    let metric = NodeRef::<html::Div>::new();
    // Caret and selection overlay rectangles, measured from the rendered text DOM
    // so inline decorations (inlay hints) and proportional glyphs stay aligned.
    let carets = RwSignal::new(Vec::<(f64, f64, f64)>::new());
    let selection_boxes = RwSignal::new(Vec::<(f64, f64, f64, f64)>::new());

    let command_set = Memo::new(move |_| {
        state
            .commands
            .get()
            .into_iter()
            .map(|command| command.method)
            .collect::<HashSet<String>>()
    });

    let buffer = move || current_buffer(state, pane_key);
    let source = move || {
        let (id, kind) = buffer();
        state.buffer_source(kind, &id)
    };
    let language = move || {
        let (id, kind) = buffer();
        match kind {
            PluginKind::File => id.as_deref().map(language_for_path).unwrap_or("plaintext"),
            _ => "rhai",
        }
    };

    let doc = RwSignal::new(Document::new(&source()));
    // Bumped on every handled key, so the scroll effect runs even for keys that
    // only move the view (`zz`/`zt`/`zb`) without changing the document.
    let tick = RwSignal::new(0_u32);

    // Reset the document when the buffer text changes from the outside (a tab
    // switch, an LSP edit), but not from our own edits, which already match.
    Effect::new(move |_| {
        let text = source();
        if doc.with_untracked(|document| document.text()) != text {
            doc.set(Document::new(&text));
        }
    });

    // Persist an edit and keep the language server in step.
    let persist = move || {
        let (id, kind) = buffer();
        let text = doc.with_untracked(|document| document.text());
        state.set_buffer_text(kind, &id, text);
        if kind == PluginKind::File
            && let Some(path) = id
        {
            crate::lsp::did_change(state, &path);
        }
    };

    let edit = move |change: &dyn Fn(&mut Document)| {
        doc.update(change);
        persist();
    };

    let cell = move || metrics(metric.get().map(Into::into));

    let on_keydown = move |event: KeyboardEvent| {
        let key = event.key();
        if matches!(
            key.as_str(),
            "Shift" | "Control" | "Alt" | "Meta" | "CapsLock" | "AltGraph"
        ) {
            return;
        }
        tick.update(|count| *count = count.wrapping_add(1));
        // The editor plugins (the modal layer) get first crack at the key, run
        // against the document; only an unconsumed key falls through to the
        // built-in navigation (so insert-mode typing reaches the hidden input).
        if crate::editor_plugins::any_enabled(state) {
            let key_event = crate::editor_plugins::KeyEvent {
                key: key.clone(),
                ctrl: event.ctrl_key(),
                shift: event.shift_key(),
                alt: event.alt_key(),
            };
            let mut outcome = crate::editor_plugins::KeyOutcome {
                consumed: false,
                changed: false,
            };
            doc.update(|document| {
                outcome = crate::editor_plugins::handle_key_document(state, document, &key_event);
            });
            if outcome.changed {
                persist();
            }
            if outcome.consumed {
                event.prevent_default();
                return;
            }
        }
        let shift = event.shift_key();
        let ctrl = event.ctrl_key() || event.meta_key();
        let alt = event.alt_key();
        let mut handled = true;
        match key.as_str() {
            "ArrowDown" if ctrl && alt => edit(&move |document| document.add_cursor_below()),
            "ArrowUp" if ctrl && alt => edit(&move |document| document.add_cursor_above()),
            "d" if ctrl && alt => edit(&move |document| document.select_next_occurrence()),
            "ArrowLeft" => edit(&move |document| document.move_left(shift)),
            "ArrowRight" => edit(&move |document| document.move_right(shift)),
            "ArrowUp" => edit(&move |document| document.move_up(shift)),
            "ArrowDown" => edit(&move |document| document.move_down(shift)),
            "Home" => {
                if ctrl {
                    edit(&move |document| document.move_doc_start(shift));
                } else {
                    edit(&move |document| document.move_line_start(shift));
                }
            }
            "End" => {
                if ctrl {
                    edit(&move |document| document.move_doc_end(shift));
                } else {
                    edit(&move |document| document.move_line_end(shift));
                }
            }
            "Escape" => edit(&move |document| document.collapse()),
            "Backspace" => edit(&move |document| document.backspace()),
            "Delete" => edit(&move |document| document.delete_forward()),
            "Enter" => edit(&move |document| document.insert("\n")),
            "Tab" => edit(&move |document| document.insert("    ")),
            "a" if ctrl => {
                let len = doc.with_untracked(Document::len_chars);
                edit(&move |document| document.set_selections(vec![Selection::new(0, len)]));
            }
            _ => handled = false,
        }
        if handled {
            event.prevent_default();
        }
    };

    // Typed text and IME compositions land in the hidden textarea; pull them out,
    // insert at the carets, and clear it for the next key.
    let on_input = move |_| {
        if let Some(element) = input.get() {
            let typed = element.value();
            if !typed.is_empty() {
                element.set_value("");
                edit(&move |document| document.insert(&typed));
            }
        }
    };

    let on_pointerdown = move |event: web_sys::PointerEvent| {
        let extend = event.shift_key();
        if let Some((line, column)) = cell_at(
            content.get().map(Into::into),
            cell(),
            event.client_x(),
            event.client_y(),
        ) {
            edit(&move |document| {
                let line = line.min(document.len_lines().saturating_sub(1));
                let line_start = document.line_to_char(line);
                let offset = (line_start + column).min(document.line_end(line));
                let anchor = if extend {
                    document.primary().anchor
                } else {
                    offset
                };
                document.set_selections(vec![Selection::new(anchor, offset)]);
            });
        }
        if let Some(element) = input.get() {
            let _ = element.focus();
        }
    };

    Effect::new(move |_| {
        if state.focused_key.get() == pane_key
            && let Some(element) = input.get()
        {
            let _ = element.focus();
        }
    });

    // After each key, record the viewport for the surface's view motions and
    // scroll the caret into view, honouring any pending `zz`/`zt`/`zb` intent.
    Effect::new(move |_| {
        tick.get();
        let Some(element) = content.get() else {
            return;
        };
        let element: web_sys::HtmlElement = element.into();
        let (_, cell_height) = cell();
        let view_height = element.client_height() as f64;
        let scroll_top = element.scroll_top() as f64;
        let visible_lines = (view_height / cell_height).floor().max(1.0) as usize;
        let first_line = (scroll_top / cell_height).floor() as usize;
        crate::editor_plugins::set_doc_viewport(visible_lines, first_line);

        let caret_line =
            doc.with_untracked(|document| document.char_to_line(document.primary().head)) as f64;
        let caret_top = caret_line * cell_height;
        let target_top = if let Some(fraction) = crate::editor_plugins::take_doc_scroll_intent() {
            caret_top - (view_height - cell_height) * fraction
        } else if caret_top < scroll_top {
            caret_top
        } else if caret_top > scroll_top + view_height - cell_height {
            caret_top - (view_height - cell_height)
        } else {
            scroll_top
        };
        let target_top = target_top.max(0.0);
        if (target_top - scroll_top).abs() >= 1.0 {
            element.set_scroll_top(target_top as i32);
        }
    });

    // Measure the caret and selection rectangles from the rendered text after
    // every change. Vertical positions stay on the uniform line grid; horizontal
    // positions are measured per line, so inline decorations cannot misalign them.
    Effect::new(move |_| {
        state.editing.highlight.get();
        command_set.track();
        let (cell_width, cell_height) = cell();
        let (Some(text_element), Some(content_element)) = (text_node.get(), content.get()) else {
            return;
        };
        let text_element: web_sys::Element = text_element.into();
        let content_element: web_sys::HtmlElement = content_element.into();
        let content_rect = content_element.get_bounding_client_rect();
        let scroll_left = content_element.scroll_left() as f64;
        let measure = |line: usize, column: usize| {
            measure_x(&text_element, &content_rect, scroll_left, line, column)
        };

        doc.with(|document| {
            let mut caret_rects = Vec::new();
            let mut box_rects = Vec::new();
            for selection in document.selections() {
                let head_line = document.char_to_line(selection.head);
                let head_column = selection.head - document.line_to_char(head_line);
                if let Some(left) = measure(head_line, head_column) {
                    caret_rects.push((left, head_line as f64 * cell_height, cell_height));
                }
                if selection.is_empty() {
                    continue;
                }
                let start = selection.start();
                let end = selection.end();
                let first_line = document.char_to_line(start);
                let last_line = document.char_to_line(end);
                for line in first_line..=last_line {
                    let line_start = document.line_to_char(line);
                    let line_length = document.line_end(line) - line_start;
                    let from = if line == first_line {
                        start - line_start
                    } else {
                        0
                    };
                    let to = if line == last_line {
                        end - line_start
                    } else {
                        line_length
                    };
                    if let (Some(left), Some(right)) = (measure(line, from), measure(line, to)) {
                        let trailing = if line == last_line { 0.0 } else { cell_width };
                        box_rects.push((
                            left,
                            line as f64 * cell_height,
                            (right - left + trailing).max(2.0),
                            cell_height,
                        ));
                    }
                }
            }
            carets.set(caret_rects);
            selection_boxes.set(box_rects);
        });
    });

    view! {
        <div class="surface" on:pointerdown=on_pointerdown>
            <div class="surface-metric" node_ref=metric>"0"</div>
            <div class="surface-gutter">
                {move || {
                    let count = doc.with(|document| document.len_lines());
                    (1..=count).map(|number| view! { <div>{number}</div> }).collect_view()
                }}
            </div>
            <div class="surface-content" node_ref=content>
                <div class="surface-text" node_ref=text_node>
                    {move || {
                        state.editing.highlight.get();
                        let language = language();
                        let set = command_set.get();
                        doc.with(|document| {
                            let text = document.text();
                            text.split('\n')
                                .map(|line| {
                                    let runs = highlight(line, language, &set);
                                    view! {
                                        <div class="surface-line">
                                            {if runs.is_empty() {
                                                view! { <span>" "</span> }.into_any()
                                            } else {
                                                runs.into_iter()
                                                    .map(|(class, run)| {
                                                        view! { <span class=class>{run}</span> }
                                                    })
                                                    .collect_view()
                                                    .into_any()
                                            }}
                                        </div>
                                    }
                                })
                                .collect_view()
                        })
                    }}
                </div>
                {move || {
                    selection_boxes
                        .get()
                        .into_iter()
                        .map(|rect| {
                            let style = format!(
                                "left:{}px;top:{}px;width:{}px;height:{}px;",
                                rect.0, rect.1, rect.2, rect.3,
                            );
                            view! { <div class="surface-selection" style=style></div> }
                        })
                        .collect_view()
                }}
                {move || {
                    carets
                        .get()
                        .into_iter()
                        .map(|(left, top, height)| {
                            let style =
                                format!("left:{left}px;top:{top}px;height:{height}px;");
                            view! { <div class="surface-caret" style=style></div> }
                        })
                        .collect_view()
                }}
                <textarea
                    class="surface-input"
                    spellcheck="false"
                    node_ref=input
                    on:keydown=on_keydown
                    on:input=on_input
                />
            </div>
        </div>
    }
}

/// The width and height of one monospace cell, measured from the probe element.
fn metrics(probe: Option<web_sys::HtmlElement>) -> (f64, f64) {
    probe
        .map(|element| {
            let rect = element.get_bounding_client_rect();
            (rect.width().max(1.0), rect.height().max(1.0))
        })
        .unwrap_or((8.0, 18.0))
}

/// The `(line, column)` nearest a client point, by the monospace cell size. The
/// caller maps it to a character offset through the document.
fn cell_at(
    content: Option<web_sys::HtmlElement>,
    cell: (f64, f64),
    client_x: i32,
    client_y: i32,
) -> Option<(usize, usize)> {
    let content = content?;
    let rect = content.get_bounding_client_rect();
    let (cell_width, cell_height) = cell;
    let x = client_x as f64 - rect.left() + content.scroll_left() as f64;
    let y = client_y as f64 - rect.top() + content.scroll_top() as f64;
    let line = (y / cell_height).max(0.0) as usize;
    let column = (x / cell_width).max(0.0).round() as usize;
    Some((line, column))
}

/// The x coordinate (in the content's scroll space) of a character column on a
/// rendered line, measured with a DOM `Range` so inline decorations and any
/// non-monospace glyphs do not throw the caret and selection out of alignment.
fn measure_x(
    text_element: &web_sys::Element,
    content_rect: &web_sys::DomRect,
    scroll_left: f64,
    line: usize,
    column: usize,
) -> Option<f64> {
    let lines = text_element.query_selector_all(".surface-line").ok()?;
    let line_element: web_sys::Element = lines.item(line as u32)?.dyn_into().ok()?;
    let (node, offset) = locate_offset(&line_element, column)?;
    let range = web_sys::Range::new().ok()?;
    range.set_start(&node, offset).ok()?;
    range.collapse_with_to_start(true);
    let rect = range.get_bounding_client_rect();
    Some(rect.left() - content_rect.left() + scroll_left)
}

/// Resolves a character column on a line to the text node and its UTF-16 offset,
/// walking the line's highlight spans (selected by tag so Leptos marker nodes do
/// not throw off the count). Past the end, lands on the last node's end.
fn locate_offset(line_element: &web_sys::Element, column: usize) -> Option<(web_sys::Node, u32)> {
    let spans = line_element.query_selector_all("span").ok()?;
    let mut remaining = column;
    let mut last: Option<(web_sys::Node, u32)> = None;
    for index in 0..spans.length() {
        let span = spans.item(index)?;
        let Some(text) = span.first_child() else {
            continue;
        };
        let content = text.text_content().unwrap_or_default();
        let char_count = content.chars().count();
        if remaining <= char_count {
            let utf16_offset = content
                .chars()
                .take(remaining)
                .map(char::len_utf16)
                .sum::<usize>() as u32;
            return Some((text, utf16_offset));
        }
        remaining -= char_count;
        last = Some((text, content.encode_utf16().count() as u32));
    }
    last
}
