//! The help and keybindings overlay, neon's `SPC ?`. Lists the default
//! keybindings and every command. Opened by `SPC ?`, F1, or the palette.

use leptos::prelude::*;

use crate::commands::palette_items;
use crate::state::EditorState;

const SECTIONS: &[(&str, &[(&str, &str)])] = &[
    (
        "Global",
        &[
            ("Ctrl+Shift+P", "Command palette"),
            ("F1", "This help"),
            ("Tab", "Indent"),
        ],
    ),
    (
        "Editing (Spacemacs / Vim)",
        &[
            ("i / a / A", "Insert before / after / at line end"),
            ("o", "Open a line below and insert"),
            ("Esc", "Back to normal mode"),
            ("h j k l", "Move left down up right"),
            ("0 / $", "Line start / end"),
            ("w / b", "Word forward / back"),
            ("x", "Delete character"),
            ("dd", "Delete line"),
        ],
    ),
    (
        "Leader (SPC)",
        &[
            ("SPC", "Open the leader menu (which-key)"),
            ("SPC SPC", "Command palette"),
            ("SPC /", "Search the project"),
            ("SPC ;", "Toggle comment on the line"),
            (
                "SPC f f / s / S / t",
                "Open folder / save / save all / file tree",
            ),
            (
                "SPC b b / d / n / p",
                "Buffer / close tab / next / previous",
            ),
            (
                "SPC w v / s / d / w",
                "Split right / below / close / focus other",
            ),
            ("SPC w h / l / =", "Focus previous / next / balance splits"),
            ("SPC s s / p", "Find in buffer / search project"),
            ("SPC j j / w / l", "Jump to char / word / line"),
            (
                "SPC t p / c / r / a / l",
                "Toggle preview / console / reference / Claude / LSP log",
            ),
            ("SPC p n / m / i", "New plugin / manager / installed"),
            ("SPC r", "Run or pause"),
            ("SPC T", "Next theme"),
            ("SPC ?", "This help"),
            (":", "Command palette"),
        ],
    ),
];

#[component]
pub fn Help(state: EditorState) -> impl IntoView {
    view! {
        <Show when=move || state.help_open.get() fallback=|| ()>
            <div class="help-overlay" on:click=move |_| state.help_open.set(false)>
                <div class="help" on:click=move |event| event.stop_propagation()>
                    <div class="help-header">
                        <span class="help-title">"Keybindings and help"</span>
                        <span class="help-dismiss">"Esc to close"</span>
                    </div>
                    <div class="help-scroll">
                        {SECTIONS
                            .iter()
                            .map(|(title, bindings)| {
                                view! {
                                    <div class="help-group">
                                        <div class="help-section">{*title}</div>
                                        {bindings
                                            .iter()
                                            .map(|(keys, description)| {
                                                view! {
                                                    <div class="help-row">
                                                        <span class="help-keys">{*keys}</span>
                                                        <span class="help-desc">{*description}</span>
                                                    </div>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                }
                            })
                            .collect_view()}
                        <div class="help-section">"Commands"</div>
                        {move || {
                            palette_items(state)
                                .into_iter()
                                .map(|(title, _)| {
                                    view! {
                                        <div class="help-row">
                                            <span class="help-desc">{title}</span>
                                        </div>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                </div>
            </div>
        </Show>
    }
}
