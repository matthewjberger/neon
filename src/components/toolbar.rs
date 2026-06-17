//! The top bar, styled like an editor menu bar: a brand, the File/Edit/View/Go/
//! Run/Help menus that dispatch editor commands, a centered command center that
//! opens the palette, and a compact right side with the renderer stats, the
//! theme picker, and the Claude toggle.

use leptos::prelude::*;

use crate::bridge::Bridge;
use crate::commands::{self, EditorCommand};
use crate::state::{EditorState, basename};
use crate::theme::{self, THEMES};

type MenuItem = (&'static str, EditorCommand);

fn menus() -> Vec<(&'static str, Vec<MenuItem>)> {
    vec![
        (
            "File",
            vec![
                ("New plugin", EditorCommand::NewPlugin),
                ("Open folder", EditorCommand::OpenFolder),
                ("Save", EditorCommand::SaveFile),
                ("Save all", EditorCommand::SaveAll),
                ("Close tab", EditorCommand::CloseTab),
            ],
        ),
        (
            "Edit",
            vec![
                ("Find and replace", EditorCommand::Find),
                ("Command palette", EditorCommand::OpenPalette),
            ],
        ),
        (
            "View",
            vec![
                ("Files", EditorCommand::ShowFiles),
                ("Search", EditorCommand::ShowSearch),
                ("Installed plugins", EditorCommand::ShowInstalled),
                ("Plugin manager", EditorCommand::ShowManager),
                ("Toggle 3D preview", EditorCommand::TogglePreview),
                ("Toggle console", EditorCommand::ToggleConsole),
                ("Toggle reference", EditorCommand::ToggleReference),
                ("Toggle control panel", EditorCommand::ToggleControlPanel),
                ("Toggle terminal", EditorCommand::ToggleTerminal),
            ],
        ),
        (
            "Go",
            vec![
                ("Jump to word", EditorCommand::JumpWord),
                ("Jump to line", EditorCommand::JumpLine),
                ("Jump to char", EditorCommand::JumpChar),
                ("Next tab", EditorCommand::NextTab),
                ("Previous tab", EditorCommand::PrevTab),
            ],
        ),
        (
            "Run",
            vec![
                ("Run or pause", EditorCommand::RunPause),
                ("Reset scene", EditorCommand::ResetScene),
            ],
        ),
        (
            "Help",
            vec![
                ("Keybindings", EditorCommand::OpenHelp),
                ("Reference", EditorCommand::ToggleReference),
            ],
        ),
    ]
}

#[component]
pub fn Toolbar(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    state: EditorState,
) -> impl IntoView {
    let open_menu = RwSignal::new(None::<&'static str>);
    let theme_open = RwSignal::new(false);

    view! {
        <div class="toolbar" on:mouseleave=move |_| open_menu.set(None)>
            <span class="brand">"Neon"</span>
            <div class="menu-bar">
                {menus()
                    .into_iter()
                    .map(|(title, items)| {
                        view! {
                            <div class="menu">
                                <button
                                    class="menu-title"
                                    class:active=move || open_menu.get() == Some(title)
                                    on:click=move |_| {
                                        open_menu
                                            .update(|current| {
                                                *current = if *current == Some(title) {
                                                    None
                                                } else {
                                                    Some(title)
                                                };
                                            });
                                    }
                                    on:mouseenter=move |_| {
                                        if open_menu.get().is_some() {
                                            open_menu.set(Some(title));
                                        }
                                    }
                                >
                                    {title}
                                </button>
                                <div
                                    class="menu-dropdown"
                                    class:open=move || open_menu.get() == Some(title)
                                >
                                    {items
                                        .into_iter()
                                        .map(|(label, command)| {
                                            view! {
                                                <div
                                                    class="menu-item"
                                                    on:click=move |_| {
                                                        open_menu.set(None);
                                                        commands::run(command.clone(), state, bridge);
                                                    }
                                                >
                                                    {label}
                                                </div>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
            <button
                class="command-center"
                title="Search and run a command"
                on:click=move |_| state.palette_open.set(true)
            >
                <span class="command-center-icon">"\u{1f50d}"</span>
                <span class="command-center-label">
                    {move || {
                        state
                            .workspace_root
                            .get()
                            .map(|root| basename(&root).to_string())
                            .unwrap_or_else(|| "neon".to_string())
                    }}
                </span>
            </button>
            <Show
                when=move || state.editor_plugins.get().iter().any(|plugin| plugin.enabled)
                fallback=|| ()
            >
                <span class="stat mode-chip">{move || state.editor_mode.get()}</span>
            </Show>
            <Show when=move || !state.status.get().is_empty() fallback=|| ()>
                <span class="stat">{move || state.status.get()}</span>
            </Show>
            <span class="stat">{move || format!("{:.0} fps", state.fps.get())}</span>
            <span class="stat">{move || format!("{} entities", state.entity_count.get())}</span>
            <div class="theme-picker">
                <button
                    class="menu-title"
                    on:click=move |_| theme_open.update(|open| *open = !*open)
                >
                    {move || theme::theme_label(&state.theme.get())}
                    " \u{25be}"
                </button>
                <Show when=move || theme_open.get() fallback=|| ()>
                    <div
                        class="theme-menu"
                        on:mouseleave=move |_| theme::preview_theme(&state.theme.get_untracked())
                    >
                        <For each=move || THEMES.to_vec() key=|(id, _)| id.to_string() let:item>
                            {
                                let id = item.0;
                                let label = item.1;
                                view! {
                                    <div
                                        class="theme-option"
                                        class:active=move || state.theme.get() == id
                                        on:mouseenter=move |_| theme::preview_theme(id)
                                        on:click=move |_| {
                                            state.theme.set(id.to_string());
                                            theme_open.set(false);
                                        }
                                    >
                                        {label}
                                    </div>
                                }
                            }
                        </For>
                    </div>
                </Show>
            </div>
            <button
                class="menu-title claude-toggle"
                class:active=move || state.chat_open.get()
                title="Ask Claude to drive the editor"
                on:click=move |_| state.chat_open.update(|open| *open = !*open)
            >
                "\u{2726} Claude"
            </button>
        </div>
    }
}
