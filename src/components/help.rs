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
            (
                "Ctrl+Alt+Up / Down",
                "Add cursor above / below (Esc clears)",
            ),
        ],
    ),
    (
        "Editing (Spacemacs / Vim)",
        &[
            (
                "i / a / A / I",
                "Insert before / after / line end / first non-blank",
            ),
            ("o", "Open a line below and insert"),
            ("Esc", "Back to normal mode"),
            ("h j k l", "Move left down up right"),
            ("0 / $", "Line start / end"),
            ("w / b", "Word forward / back"),
            ("/", "Find in buffer"),
            ("x / X", "Delete character forward / back"),
            ("D / C", "Delete / change to line end"),
            ("J", "Join the next line up"),
            ("dd", "Delete line"),
        ],
    ),
    (
        "Leader (SPC)",
        &[
            ("SPC", "Open the leader menu (which-key)"),
            ("SPC SPC", "Command palette"),
            ("SPC TAB", "Last buffer (previous tab)"),
            ("SPC /", "Search the project"),
            ("SPC ;", "Toggle comment on the line"),
            ("SPC ?", "This help"),
            (
                "SPC f f / s / S / t / n",
                "Open folder / save / save all / tree / new",
            ),
            ("SPC b b / d / n / p", "List / close / next / previous tab"),
            ("SPC w v / s / d", "Split right / split below / close split"),
            (
                "SPC w h / l / w / =",
                "Focus previous / next / other / balance",
            ),
            (
                "SPC s s / p / j",
                "Find in buffer / search project / jump word",
            ),
            ("SPC j j / l / w / i", "Jump to char / line / word / symbol"),
            ("SPC j + / =", "Format the buffer (rust-analyzer)"),
            (
                "SPC g g / t / i / r",
                "Definition (also gd) / type / implementation / references",
            ),
            ("SPC g R / s / S", "Rename / symbol / workspace symbols"),
            (
                "SPC h h / s / k / t",
                "Hover (also K) / signature help / keybindings / tour",
            ),
            ("SPC a", "Code action (also SPC x .)"),
            (
                "SPC c c / b / t / r / k / o",
                "Cargo check / build / test / run / cancel / output",
            ),
            (
                "SPC x ; / d / j / k / J",
                "Comment / duplicate / move down / up / join",
            ),
            ("SPC x > / <", "Indent / outdent"),
            (
                "SPC x u / U / s / w / r",
                "Lower / upper case / sort lines / trim / rename",
            ),
            (
                "SPC t p / c / r / a / o / l",
                "Toggle preview / console / reference / Claude / control panel / LSP log",
            ),
            (
                "SPC e n / p / e / l",
                "Next / previous error / problems list / rust-analyzer log",
            ),
            ("SPC p f / t", "Search project / file tree"),
            ("SPC P n / m / i", "New plugin / manager / installed"),
            ("SPC r", "Run or pause"),
            ("SPC T", "Next theme"),
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
