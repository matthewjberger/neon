//! The terminal panel: renders the emulator's screen grid and a cursor, and
//! captures keystrokes, encoding them to the bytes a terminal expects. It opens
//! the PTY at the measured grid size when shown and resizes with the panel.

use leptos::html;
use leptos::prelude::*;
use web_sys::HtmlElement;

use crate::state::EditorState;

#[component]
pub fn Terminal(state: EditorState) -> impl IntoView {
    let grid_ref = NodeRef::<html::Div>::new();
    Effect::new(move |_| {
        if state.terminal.open.get()
            && state.terminal.connected.get()
            && let Some(element) = grid_ref.get()
        {
            let _ = element.focus();
            let (cols, rows) = measure(&element);
            crate::terminal::open(state, cols, rows);
        }
    });
    let _ = window_event_listener(leptos::ev::resize, move |_| {
        if state.terminal.open.get_untracked()
            && let Some(element) = grid_ref.get_untracked()
        {
            let (cols, rows) = measure(&element);
            crate::terminal::resize(cols, rows);
        }
    });
    let on_keydown = move |event: web_sys::KeyboardEvent| {
        if let Some(bytes) = key_to_bytes(&event) {
            event.prevent_default();
            crate::terminal::send_input(bytes);
        }
    };
    view! {
        <Show when=move || state.terminal.open.get() fallback=|| ()>
            <div class="terminal-panel">
                <div class="terminal-header">
                    <span>"Terminal"</span>
                    <span class="terminal-actions">
                        <button class="icon-button" on:click=move |_| crate::terminal::interrupt()>
                            "^C"
                        </button>
                        <button class="icon-button" on:click=move |_| state.terminal.open.set(false)>
                            "x"
                        </button>
                    </span>
                </div>
                <div class="terminal-grid" tabindex="0" node_ref=grid_ref on:keydown=on_keydown>
                    {move || {
                        let Some(grid) = state.terminal.grid.get() else {
                            return ().into_any();
                        };
                        let rows = grid
                            .lines
                            .into_iter()
                            .map(|spans| {
                                let cells = spans
                                    .into_iter()
                                    .map(|span| {
                                        let style = span_style(&span);
                                        view! { <span style=style>{span.text}</span> }
                                    })
                                    .collect_view();
                                view! { <div class="term-row">{cells}</div> }
                            })
                            .collect_view();
                        let cursor = grid.cursor_visible.then(|| {
                            let style = format!(
                                "left:calc({} * 1ch);top:calc({} * 1.2em)",
                                grid.cursor_col, grid.cursor_row,
                            );
                            view! { <div class="term-cursor" style=style></div> }
                        });
                        view! { {rows} {cursor} }.into_any()
                    }}
                </div>
            </div>
        </Show>
    }
}

fn span_style(span: &protocol::TermSpan) -> String {
    let mut style = String::new();
    let (fg, bg) = if span.inverse {
        (
            if span.bg.is_empty() {
                "var(--bg)"
            } else {
                &span.bg
            },
            if span.fg.is_empty() {
                "var(--text)"
            } else {
                &span.fg
            },
        )
    } else {
        (span.fg.as_str(), span.bg.as_str())
    };
    if !fg.is_empty() {
        style.push_str(&format!("color:{fg};"));
    }
    if !bg.is_empty() {
        style.push_str(&format!("background:{bg};"));
    }
    if span.bold {
        style.push_str("font-weight:bold;");
    }
    if span.italic {
        style.push_str("font-style:italic;");
    }
    if span.underline {
        style.push_str("text-decoration:underline;");
    }
    style
}

/// Computes the grid size from the panel's pixel size and the monospace font.
fn measure(element: &HtmlElement) -> (u16, u16) {
    let font_size = web_sys::window()
        .and_then(|window| window.get_computed_style(element).ok().flatten())
        .and_then(|style| style.get_property_value("font-size").ok())
        .and_then(|value| value.trim_end_matches("px").trim().parse::<f64>().ok())
        .unwrap_or(13.0);
    let char_width = (font_size * 0.6).max(1.0);
    let line_height = (font_size * 1.2).max(1.0);
    let width = element.client_width() as f64;
    let height = element.client_height() as f64;
    let cols = (width / char_width).floor().clamp(1.0, 400.0) as u16;
    let rows = (height / line_height).floor().clamp(1.0, 200.0) as u16;
    (cols, rows)
}

/// Encodes a keystroke into the bytes a terminal expects, or None to ignore it.
fn key_to_bytes(event: &web_sys::KeyboardEvent) -> Option<Vec<u8>> {
    if event.meta_key() {
        return None;
    }
    let key = event.key();
    let bytes = match key.as_str() {
        "Enter" => vec![b'\r'],
        "Backspace" => vec![0x7f],
        "Tab" => vec![b'\t'],
        "Escape" => vec![0x1b],
        "ArrowUp" => b"\x1b[A".to_vec(),
        "ArrowDown" => b"\x1b[B".to_vec(),
        "ArrowRight" => b"\x1b[C".to_vec(),
        "ArrowLeft" => b"\x1b[D".to_vec(),
        "Home" => b"\x1b[H".to_vec(),
        "End" => b"\x1b[F".to_vec(),
        "Delete" => b"\x1b[3~".to_vec(),
        "PageUp" => b"\x1b[5~".to_vec(),
        "PageDown" => b"\x1b[6~".to_vec(),
        _ => {
            if key.chars().count() != 1 {
                return None;
            }
            let character = key.chars().next()?;
            if event.ctrl_key() {
                let upper = character.to_ascii_uppercase() as u8;
                return (0x40..=0x5f).contains(&upper).then(|| vec![upper & 0x1f]);
            }
            let mut bytes = Vec::new();
            if event.alt_key() {
                bytes.push(0x1b);
            }
            let mut buffer = [0_u8; 4];
            bytes.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
            bytes
        }
    };
    Some(bytes)
}
