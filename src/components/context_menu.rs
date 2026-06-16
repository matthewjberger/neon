//! The custom right-click menu. The webview's default menu is suppressed and
//! replaced by this one, anchored at the pointer, listing editor commands for
//! the thing that was clicked. A transparent backdrop closes it.

use leptos::prelude::*;

use crate::bridge::Bridge;
use crate::commands::{self, EditorCommand};
use crate::state::{ContextMenu, EditorState};

/// Opens the context menu at a pointer position with the given items.
pub fn open(state: EditorState, x: f64, y: f64, items: Vec<(String, EditorCommand)>) {
    state.context_menu.set(Some(ContextMenu { x, y, items }));
}

/// The general menu for empty chrome: panels, palette, and theme.
pub fn general_menu() -> Vec<(String, EditorCommand)> {
    vec![
        ("Command palette".into(), EditorCommand::OpenPalette),
        ("Control panel".into(), EditorCommand::ToggleControlPanel),
        ("Files".into(), EditorCommand::ShowFiles),
        ("Search project".into(), EditorCommand::ShowSearch),
        ("Toggle 3D preview".into(), EditorCommand::TogglePreview),
        ("Toggle console".into(), EditorCommand::ToggleConsole),
        ("Next theme".into(), EditorCommand::NextTheme),
        ("Keybindings".into(), EditorCommand::OpenHelp),
    ]
}

/// The menu for the editing surface.
pub fn editor_menu() -> Vec<(String, EditorCommand)> {
    vec![
        ("Find and replace".into(), EditorCommand::Find),
        ("Jump to word".into(), EditorCommand::JumpWord),
        ("Jump to line".into(), EditorCommand::JumpLine),
        ("Command palette".into(), EditorCommand::OpenPalette),
        (
            "Split right".into(),
            EditorCommand::SplitEditor { vertical: true },
        ),
        (
            "Split below".into(),
            EditorCommand::SplitEditor { vertical: false },
        ),
    ]
}

/// The menu for a tab.
pub fn tab_menu() -> Vec<(String, EditorCommand)> {
    vec![
        ("Close tab".into(), EditorCommand::CloseTab),
        ("Next tab".into(), EditorCommand::NextTab),
        ("Previous tab".into(), EditorCommand::PrevTab),
        (
            "Split right".into(),
            EditorCommand::SplitEditor { vertical: true },
        ),
        ("Close split".into(), EditorCommand::CloseSplit),
    ]
}

/// The menu for the file tree and search panel.
pub fn file_menu() -> Vec<(String, EditorCommand)> {
    vec![
        ("Open folder".into(), EditorCommand::OpenFolder),
        ("Search project".into(), EditorCommand::ShowSearch),
        ("Save all".into(), EditorCommand::SaveAll),
        ("New plugin".into(), EditorCommand::NewPlugin),
    ]
}

/// The menu for the plugin panels.
pub fn plugin_menu() -> Vec<(String, EditorCommand)> {
    vec![
        ("New plugin".into(), EditorCommand::NewPlugin),
        ("Plugin manager".into(), EditorCommand::ShowManager),
        ("Installed plugins".into(), EditorCommand::ShowInstalled),
    ]
}

#[component]
pub fn ContextMenuView(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    state: EditorState,
) -> impl IntoView {
    view! {
        <Show when=move || state.context_menu.get().is_some() fallback=|| ()>
            <div
                class="context-backdrop"
                on:click=move |_| state.context_menu.set(None)
                on:contextmenu=move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    state.context_menu.set(None);
                }
            >
                {move || {
                    let menu = state.context_menu.get();
                    menu.map(|menu| {
                        let style = format!("left:{}px;top:{}px;", menu.x, menu.y);
                        view! {
                            <div
                                class="context-menu"
                                style=style
                                on:click=move |event: web_sys::MouseEvent| event.stop_propagation()
                            >
                                {menu
                                    .items
                                    .into_iter()
                                    .map(|(label, command)| {
                                        view! {
                                            <div
                                                class="context-item"
                                                on:click=move |_| {
                                                    state.context_menu.set(None);
                                                    commands::run(command.clone(), state, bridge);
                                                }
                                            >
                                                {label}
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                    })
                }}
            </div>
        </Show>
    }
}
