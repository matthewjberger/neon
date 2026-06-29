//! The custom editing surface: a rope-backed [`Document`] rendered as DOM, with
//! native multi-cursor. It is the opt-in alternative to the textarea (toggled by
//! `state.editing.surface`). A hidden textarea captures keystrokes, IME, and
//! clipboard; the text is rendered as DOM, and the caret and selection overlays
//! are positioned by measuring the rendered glyphs with a `Range`, so inline
//! decorations stay aligned. Edits persist through `set_buffer_text`, so
//! highlighting and saving keep working. The modal plugin layer runs against the
//! document through `handle_key_document`, including macros, the dot-repeat, and
//! the view motions, so the surface is a full modal editor. The hidden textarea
//! mirrors the whole document and primary selection and registers as the LSP
//! source, so completion, hover, go-to, and rename act on the surface's caret.

use std::collections::HashSet;

use document::{Document, Selection};
use leptos::html;
use leptos::prelude::*;
use web_sys::KeyboardEvent;

use crate::highlight::highlight;
use crate::state::{EditorState, InlayHint, PluginKind, language_for_path};

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

    // The fold layout: `rows` maps each visual row to a document line, and
    // `visual_of` maps a document line to the visual row it shows on (a folded
    // line maps to its header). Recomputed when the line count or folds change.
    let fold_view = Memo::new(move |_| {
        let (id, kind) = buffer();
        let path = if kind == PluginKind::File { id } else { None };
        let folds = path
            .as_ref()
            .and_then(|path| state.editing.folds.with(|map| map.get(path).cloned()))
            .unwrap_or_default();
        let lenses = path
            .as_ref()
            .and_then(|path| state.lsp.code_lenses.with(|map| map.get(path).cloned()))
            .unwrap_or_default();
        let line_count = doc.with(|document| document.len_lines());
        build_rows(line_count, &folds, &lenses)
    });

    // Toggles the fold whose header is `header_line`, looking up its extent in the
    // server's folding ranges and recording or clearing it for the focused file.
    let toggle_fold = move |header_line: usize| {
        let (id, kind) = buffer();
        let Some(path) = (kind == PluginKind::File).then_some(id).flatten() else {
            return;
        };
        let range = state.lsp.folding_ranges.with(|map| {
            map.get(&path)
                .and_then(|ranges| {
                    ranges
                        .iter()
                        .find(|(start, _)| *start as usize == header_line)
                })
                .copied()
        });
        let Some((start, end)) = range else {
            return;
        };
        state.editing.folds.update(|map| {
            let entry = map.entry(path.clone()).or_default();
            if let Some(position) = entry.iter().position(|(s, _)| *s == start as usize) {
                entry.remove(position);
            } else {
                entry.push((start as usize, end as usize));
            }
        });
    };

    let on_keydown = move |event: KeyboardEvent| {
        let key = event.key();
        if matches!(
            key.as_str(),
            "Shift" | "Control" | "Alt" | "Meta" | "CapsLock" | "AltGraph"
        ) {
            return;
        }
        tick.update(|count| *count = count.wrapping_add(1));
        // When the completion popup is open it owns navigation and acceptance,
        // ahead of the modal layer, mirroring the textarea editor.
        if state.lsp.completion.get_untracked().is_some() {
            let len = state
                .lsp
                .completion
                .with_untracked(|menu| menu.as_ref().map(|menu| menu.items.len()).unwrap_or(0))
                .max(1);
            match key.as_str() {
                "ArrowDown" => {
                    event.prevent_default();
                    state
                        .lsp
                        .completion_index
                        .update(|index| *index = (*index + 1) % len);
                    return;
                }
                "ArrowUp" => {
                    event.prevent_default();
                    state
                        .lsp
                        .completion_index
                        .update(|index| *index = (*index + len - 1) % len);
                    return;
                }
                "Enter" | "Tab" => {
                    event.prevent_default();
                    crate::lsp::accept_completion(
                        state,
                        state.lsp.completion_index.get_untracked(),
                    );
                    return;
                }
                "Escape" => {
                    event.prevent_default();
                    state.lsp.completion.set(None);
                    return;
                }
                _ => {}
            }
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

    // The hidden textarea mirrors the whole document and its primary selection, so
    // it is the source the LSP client reads and the target completion-accept and
    // find-replace write to. Typed text, IME, paste, and those programmatic edits
    // all surface here: rebuild the document from the new value and caret.
    let on_input = move |_| {
        if let Some(element) = input.get() {
            let value = element.value();
            let caret = element.selection_start().ok().flatten().unwrap_or(0);
            let head = utf16_to_char(&value, caret);
            doc.update(|document| {
                *document = Document::new(&value);
                document.set_primary(Selection::caret(head));
            });
            persist();
            crate::lsp::request_completion(state);
        }
    };

    let on_pointerdown = move |event: web_sys::PointerEvent| {
        let extend = event.shift_key();
        let (_, cell_height) = cell();
        if let (Some(content_element), Some(text_element)) = (content.get(), text_node.get()) {
            let content_element: web_sys::HtmlElement = content_element.into();
            let text_element: web_sys::Element = text_element.into();
            let rect = content_element.get_bounding_client_rect();
            let scroll_left = content_element.scroll_left() as f64;
            let x = event.client_x() as f64 - rect.left() + scroll_left;
            let y = event.client_y() as f64 - rect.top() + content_element.scroll_top() as f64;
            let visual_row = (y / cell_height).max(0.0) as usize;
            let rows = fold_view.get().0;
            let doc_line = rows
                .get(visual_row)
                .or_else(|| rows.last())
                .map(SurfaceRow::doc_line)
                .unwrap_or(0);
            edit(&move |document| {
                let line = doc_line.min(document.len_lines().saturating_sub(1));
                let line_start = document.line_to_char(line);
                let line_length = document.line_end(line) - line_start;
                let column = column_at_x(&text_element, &rect, scroll_left, line, x, line_length);
                let offset = line_start + column;
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

        let caret_doc_line =
            doc.with_untracked(|document| document.char_to_line(document.primary().head));
        let caret_line = fold_view.with(|(_, visual_of)| {
            visual_of
                .get(caret_doc_line)
                .copied()
                .unwrap_or(caret_doc_line)
        }) as f64;
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
        let scroll_top = content_element.scroll_top() as f64;
        let measure = |line: usize, column: usize| {
            measure_x(&text_element, &content_rect, scroll_left, line, column)
        };

        let (rows, visual_of) = fold_view.get();

        doc.with(|document| {
            let mut caret_rects = Vec::new();
            let mut box_rects = Vec::new();
            for selection in document.selections() {
                let head_line = document.char_to_line(selection.head);
                let head_column = selection.head - document.line_to_char(head_line);
                if let Some(left) = measure(visual_of[head_line], head_column) {
                    caret_rects.push((
                        left,
                        visual_of[head_line] as f64 * cell_height,
                        cell_height,
                    ));
                }
                if selection.is_empty() {
                    continue;
                }
                let start = selection.start();
                let end = selection.end();
                let first_line = document.char_to_line(start);
                let last_line = document.char_to_line(end);
                for (offset, &row) in visual_of[first_line..=last_line].iter().enumerate() {
                    let line = first_line + offset;
                    if !matches!(rows.get(row), Some(SurfaceRow::Text(text_line)) if *text_line == line)
                    {
                        continue;
                    }
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
                    if let (Some(left), Some(right)) = (measure(row, from), measure(row, to)) {
                        let trailing = if line == last_line { 0.0 } else { cell_width };
                        box_rects.push((
                            left,
                            row as f64 * cell_height,
                            (right - left + trailing).max(2.0),
                            cell_height,
                        ));
                    }
                }
            }
            carets.set(caret_rects);
            selection_boxes.set(box_rects);

            let primary = document.primary().head;
            let primary_line = document.char_to_line(primary);
            let primary_column = primary - document.line_to_char(primary_line);
            let pixel = measure(visual_of[primary_line], primary_column).map(|left| {
                let viewport_x = content_rect.left() + left - scroll_left;
                let viewport_y = content_rect.top() + visual_of[primary_line] as f64 * cell_height
                    - scroll_top
                    + cell_height;
                (viewport_x, viewport_y)
            });
            state.editing.surface_caret_pixel.set(pixel);
        });
    });

    // Mirror the document and primary selection into the hidden textarea, and
    // register it as the LSP source. With the whole buffer and caret reflected
    // there, the existing LSP requests read the surface's caret unchanged.
    Effect::new(move |_| {
        let Some(element) = input.get() else {
            return;
        };
        let (text, start, end) = doc.with(|document| {
            let selection = document.primary();
            (document.text(), selection.start(), selection.end())
        });
        if element.value() != text {
            element.set_value(&text);
        }
        let low = char_to_utf16(&text, start);
        let high = char_to_utf16(&text, end);
        let _ = element.set_selection_range(low, high);
        crate::components::overlays::find::set_active(element);
    });

    view! {
        <div class="surface" on:pointerdown=on_pointerdown>
            <div class="surface-metric" node_ref=metric>"0"</div>
            <div class="surface-gutter">
                {move || {
                    let rows = fold_view.get().0;
                    let (id, kind) = buffer();
                    let path = if kind == PluginKind::File { id } else { None };
                    let foldable = path
                        .as_ref()
                        .map(|path| {
                            state.lsp.folding_ranges.with(|map| {
                                map.get(path)
                                    .map(|ranges| {
                                        ranges.iter().map(|(start, _)| *start as usize).collect()
                                    })
                                    .unwrap_or_default()
                            })
                        })
                        .unwrap_or_else(HashSet::new);
                    let folded = path
                        .as_ref()
                        .map(|path| {
                            state.editing.folds.with(|map| {
                                map.get(path)
                                    .map(|folds| folds.iter().map(|(start, _)| *start).collect())
                                    .unwrap_or_default()
                            })
                        })
                        .unwrap_or_else(HashSet::new);
                    rows.into_iter()
                        .map(|row| match row {
                            SurfaceRow::Lens(..) => view! {
                                <div class="surface-gutter-row"></div>
                            }
                            .into_any(),
                            SurfaceRow::Text(doc_line) => {
                                let marker = if !foldable.contains(&doc_line) {
                                    ""
                                } else if folded.contains(&doc_line) {
                                    "\u{25b8}"
                                } else {
                                    "\u{25be}"
                                };
                                let toggle = move |_| toggle_fold(doc_line);
                                view! {
                                    <div class="surface-gutter-row">
                                        <span class="surface-fold" on:click=toggle>{marker}</span>
                                        <span>{doc_line + 1}</span>
                                    </div>
                                }
                                .into_any()
                            }
                        })
                        .collect_view()
                }}
            </div>
            <div class="surface-content" node_ref=content>
                <div class="surface-text" node_ref=text_node>
                    {move || {
                        state.editing.highlight.get();
                        let language = language();
                        let set = command_set.get();
                        let (id, kind) = buffer();
                        let hints = if kind == PluginKind::File {
                            id.and_then(|path| {
                                    state.lsp.inlay_hints.with(|map| map.get(&path).cloned())
                                })
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        };
                        let rows = fold_view.get().0;
                        doc.with(|document| {
                            let text = document.text();
                            let lines = text.split('\n').collect::<Vec<_>>();
                            rows.into_iter()
                                .map(|row| match row {
                                    SurfaceRow::Lens(_, title) => view! {
                                        <div class="surface-line surface-lens">
                                            <span class="surface-lens-text">{title}</span>
                                        </div>
                                    }
                                    .into_any(),
                                    SurfaceRow::Text(doc_line) => {
                                        let line = lines.get(doc_line).copied().unwrap_or("");
                                        let line_hints = hints
                                            .iter()
                                            .filter(|hint| hint.line as usize == doc_line)
                                            .cloned()
                                            .collect::<Vec<_>>();
                                        render_line(doc_line, line, language, &set, &line_hints)
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

/// Converts a UTF-16 offset (the textarea's unit) to a character offset (the
/// document's unit).
fn utf16_to_char(value: &str, utf16_offset: u32) -> usize {
    let mut units = 0_u32;
    for (index, character) in value.chars().enumerate() {
        if units >= utf16_offset {
            return index;
        }
        units += character.len_utf16() as u32;
    }
    value.chars().count()
}

/// Converts a character offset to a UTF-16 offset, for the textarea selection.
fn char_to_utf16(value: &str, char_offset: usize) -> u32 {
    value
        .chars()
        .take(char_offset)
        .map(|character| character.len_utf16() as u32)
        .sum()
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

/// The character column on a line whose measured x is nearest a click, found by
/// bisecting the (monotonic) measured positions so a click lands right even when
/// inlay hints have shifted the glyphs.
fn column_at_x(
    text_element: &web_sys::Element,
    content_rect: &web_sys::DomRect,
    scroll_left: f64,
    line: usize,
    target_x: f64,
    line_length: usize,
) -> usize {
    if line_length == 0 {
        return 0;
    }
    let at = |column: usize| measure_x(text_element, content_rect, scroll_left, line, column);
    let mut low = 0_usize;
    let mut high = line_length;
    while low < high {
        let mid = low.midpoint(high + 1);
        match at(mid) {
            Some(x) if x <= target_x => low = mid,
            _ => high = mid - 1,
        }
    }
    let mut best = low;
    let mut best_distance = f64::MAX;
    for column in [low, (low + 1).min(line_length)] {
        if let Some(x) = at(column) {
            let distance = (x - target_x).abs();
            if distance < best_distance {
                best_distance = distance;
                best = column;
            }
        }
    }
    best
}

/// A visual row on the surface: a document line of text, or a code-lens label
/// drawn above the line it annotates.
#[derive(Clone, PartialEq)]
enum SurfaceRow {
    Text(usize),
    Lens(usize, String),
}

impl SurfaceRow {
    /// The document line this row sits on (a lens shares its annotated line).
    fn doc_line(&self) -> usize {
        match self {
            SurfaceRow::Text(line) | SurfaceRow::Lens(line, _) => *line,
        }
    }
}

/// The visual layout for a document. Returns `rows`, the ordered visual rows
/// (text and lens), and `visual_of`, the visual-row index of each document line's
/// text row (a folded line maps to the row of its header).
fn build_rows(
    line_count: usize,
    folds: &[(usize, usize)],
    lenses: &[(u32, String)],
) -> (Vec<SurfaceRow>, Vec<usize>) {
    let mut hidden = vec![false; line_count];
    for &(start, end) in folds {
        let upper = end.min(line_count.saturating_sub(1));
        if start < upper {
            hidden[(start + 1)..=upper].fill(true);
        }
    }
    let mut rows = Vec::new();
    let mut visual_of = vec![0_usize; line_count];
    let mut last_visual = 0_usize;
    for line in 0..line_count {
        if hidden[line] {
            visual_of[line] = last_visual;
            continue;
        }
        for (lens_line, title) in lenses {
            if *lens_line as usize == line {
                rows.push(SurfaceRow::Lens(line, title.clone()));
            }
        }
        last_visual = rows.len();
        visual_of[line] = rows.len();
        rows.push(SurfaceRow::Text(line));
    }
    (rows, visual_of)
}

/// Renders one line: its highlight runs with the line's inlay hints woven in at
/// their columns as non-editable spans. Hint spans carry `surface-inlay`, which
/// the column measurement skips, so they shift glyphs visually without moving
/// the caret's character math.
fn render_line(
    doc_line: usize,
    line: &str,
    language: &str,
    commands: &HashSet<String>,
    hints: &[InlayHint],
) -> AnyView {
    let runs = highlight(line, language, commands);
    let mut sorted = hints.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|hint| hint.character);
    let mut elements: Vec<AnyView> = Vec::new();
    let mut column = 0_usize;
    let mut next_hint = 0_usize;
    for (class, run) in runs {
        let run_chars = run.chars().collect::<Vec<_>>();
        let run_end = column + run_chars.len();
        let mut local = 0_usize;
        while next_hint < sorted.len() && (sorted[next_hint].character as usize) < run_end {
            let split = (sorted[next_hint].character as usize).saturating_sub(column);
            if split >= local {
                let segment = run_chars[local..split].iter().collect::<String>();
                if !segment.is_empty() {
                    elements.push(view! { <span class=class>{segment}</span> }.into_any());
                }
                local = split;
                elements.push(inlay_span(sorted[next_hint]));
            }
            next_hint += 1;
        }
        let rest = run_chars[local..].iter().collect::<String>();
        if !rest.is_empty() {
            elements.push(view! { <span class=class>{rest}</span> }.into_any());
        }
        column = run_end;
    }
    while next_hint < sorted.len() {
        elements.push(inlay_span(sorted[next_hint]));
        next_hint += 1;
    }
    if elements.is_empty() {
        elements.push(view! { <span>" "</span> }.into_any());
    }
    view! { <div class="surface-line" data-doc=doc_line.to_string()>{elements}</div> }.into_any()
}

/// A non-editable inlay-hint span, with the padding the server requested as
/// surrounding spaces.
fn inlay_span(hint: &InlayHint) -> AnyView {
    let mut label = hint.label.clone();
    if hint.padding_left {
        label = format!(" {label}");
    }
    if hint.padding_right {
        label = format!("{label} ");
    }
    view! { <span class="surface-inlay">{label}</span> }.into_any()
}

/// The x coordinate (in the content's scroll space) of a character column on a
/// rendered line, measured with a DOM `Range` so inline decorations and any
/// non-monospace glyphs do not throw the caret and selection out of alignment.
fn measure_x(
    text_element: &web_sys::Element,
    content_rect: &web_sys::DomRect,
    scroll_left: f64,
    doc_line: usize,
    column: usize,
) -> Option<f64> {
    let line_element = text_element
        .query_selector(&format!(".surface-line[data-doc=\"{doc_line}\"]"))
        .ok()??;
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
    let spans = line_element
        .query_selector_all("span:not(.surface-inlay)")
        .ok()?;
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
