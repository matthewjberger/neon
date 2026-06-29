//! The custom editing surface: a rope-backed [`Document`] rendered as DOM, with
//! native multi-cursor. It is the opt-in alternative to the textarea (toggled by
//! `state.editing.surface`). A hidden textarea captures keystrokes, IME, and
//! clipboard; the text, carets, and selections are rendered as overlays measured
//! against a monospace cell. Edits persist through `set_buffer_text`, so
//! highlighting and saving keep working. The vim plugin layer is not yet wired to
//! it, so the surface is a plain editor for now.

use std::collections::HashSet;

use document::{Document, Selection};
use leptos::html;
use leptos::prelude::*;
use web_sys::KeyboardEvent;

use crate::highlight::highlight;
use crate::state::{EditorState, PluginKind, language_for_path};

use super::current_buffer;

#[component]
pub fn CodeSurface(state: EditorState, pane_key: usize) -> impl IntoView {
    let input = NodeRef::<html::Textarea>::new();
    let content = NodeRef::<html::Div>::new();
    let metric = NodeRef::<html::Div>::new();

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
                <div class="surface-text">
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
                    let (cell_width, cell_height) = cell();
                    doc.with(|document| {
                        document
                            .selections()
                            .iter()
                            .flat_map(|selection| {
                                selection_rects(document, selection, cell_width, cell_height)
                            })
                            .map(|rect| {
                                let style = format!(
                                    "left:{}px;top:{}px;width:{}px;height:{}px;",
                                    rect.0, rect.1, rect.2, rect.3,
                                );
                                view! { <div class="surface-selection" style=style></div> }
                            })
                            .collect_view()
                    })
                }}
                {move || {
                    let (cell_width, cell_height) = cell();
                    doc.with(|document| {
                        document
                            .selections()
                            .iter()
                            .map(|selection| {
                                let line = document.char_to_line(selection.head);
                                let column = selection.head - document.line_to_char(line);
                                let style = format!(
                                    "left:{}px;top:{}px;height:{}px;",
                                    column as f64 * cell_width,
                                    line as f64 * cell_height,
                                    cell_height,
                                );
                                view! { <div class="surface-caret" style=style></div> }
                            })
                            .collect_view()
                    })
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

/// The highlight rectangles a selection covers, one per line it spans.
fn selection_rects(
    document: &Document,
    selection: &Selection,
    cell_width: f64,
    cell_height: f64,
) -> Vec<(f64, f64, f64, f64)> {
    if selection.is_empty() {
        return Vec::new();
    }
    let start = selection.start();
    let end = selection.end();
    let first_line = document.char_to_line(start);
    let last_line = document.char_to_line(end);
    let mut rects = Vec::new();
    for line in first_line..=last_line {
        let line_start = document.line_to_char(line);
        let line_end = document.line_end(line);
        let from = start.max(line_start) - line_start;
        let to = if line == last_line {
            end - line_start
        } else {
            (line_end - line_start) + 1
        };
        let left = from as f64 * cell_width;
        let width = (to.saturating_sub(from)) as f64 * cell_width;
        rects.push((left, line as f64 * cell_height, width.max(2.0), cell_height));
    }
    rects
}
