//! The editor operations registry. Each command is a named editor action the
//! command palette lists and an editor plugin can invoke by id (the way vim's
//! `:` and Spacemacs' SPC menus drive the editor). One place defines what the
//! editor can do.

use leptos::prelude::*;

use crate::bridge::{self, Bridge};
use crate::plugins;
use crate::state::{EditorState, PluginKind, SidebarView};
use crate::theme::THEMES;

/// One editor operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorCommand {
    SplitEditor { vertical: bool },
    CloseSplit,
    FocusOther,
    TogglePreview,
    ToggleConsole,
    ToggleReference,
    ToggleChat,
    ShowInstalled,
    ShowManager,
    ShowFiles,
    ShowSearch,
    OpenFolder,
    SaveFile,
    SaveAll,
    CloseTab,
    NextTab,
    PrevTab,
    FocusNext,
    FocusPrev,
    BalanceSplits,
    Find,
    JumpWord,
    JumpLine,
    JumpChar,
    ToggleLspLog,
    NewPlugin,
    RunPause,
    ResetScene,
    NextTheme,
    OpenPalette,
    OpenHelp,
    SetTheme(String),
    OpenBuffer { kind: PluginKind, id: String },
}

/// The id an editor plugin uses to invoke a static command.
pub fn command_from_id(id: &str) -> Option<EditorCommand> {
    Some(match id {
        "split-right" => EditorCommand::SplitEditor { vertical: true },
        "split-below" => EditorCommand::SplitEditor { vertical: false },
        "close-split" => EditorCommand::CloseSplit,
        "focus-other" => EditorCommand::FocusOther,
        "toggle-preview" => EditorCommand::TogglePreview,
        "toggle-console" => EditorCommand::ToggleConsole,
        "toggle-reference" => EditorCommand::ToggleReference,
        "toggle-chat" => EditorCommand::ToggleChat,
        "show-installed" => EditorCommand::ShowInstalled,
        "show-manager" => EditorCommand::ShowManager,
        "show-files" => EditorCommand::ShowFiles,
        "show-search" => EditorCommand::ShowSearch,
        "open-folder" => EditorCommand::OpenFolder,
        "save-file" => EditorCommand::SaveFile,
        "save-all" => EditorCommand::SaveAll,
        "close-tab" => EditorCommand::CloseTab,
        "next-tab" => EditorCommand::NextTab,
        "prev-tab" => EditorCommand::PrevTab,
        "focus-next" => EditorCommand::FocusNext,
        "focus-prev" => EditorCommand::FocusPrev,
        "balance-splits" => EditorCommand::BalanceSplits,
        "find" => EditorCommand::Find,
        "jump-word" => EditorCommand::JumpWord,
        "jump-line" => EditorCommand::JumpLine,
        "jump-char" => EditorCommand::JumpChar,
        "toggle-lsp-log" => EditorCommand::ToggleLspLog,
        "new-plugin" => EditorCommand::NewPlugin,
        "run-pause" => EditorCommand::RunPause,
        "reset-scene" => EditorCommand::ResetScene,
        "next-theme" => EditorCommand::NextTheme,
        "open-palette" => EditorCommand::OpenPalette,
        "open-help" => EditorCommand::OpenHelp,
        _ => return None,
    })
}

/// Every command the palette offers: the static operations, a theme per theme,
/// and an open command per installed plugin and built-in module.
pub fn palette_items(state: EditorState) -> Vec<(String, EditorCommand)> {
    let mut items = vec![
        (
            "Split right".to_string(),
            EditorCommand::SplitEditor { vertical: true },
        ),
        (
            "Split below".to_string(),
            EditorCommand::SplitEditor { vertical: false },
        ),
        ("Close split".to_string(), EditorCommand::CloseSplit),
        ("Focus other pane".to_string(), EditorCommand::FocusOther),
        (
            "Toggle 3D preview".to_string(),
            EditorCommand::TogglePreview,
        ),
        ("Toggle console".to_string(), EditorCommand::ToggleConsole),
        (
            "Toggle reference".to_string(),
            EditorCommand::ToggleReference,
        ),
        ("Toggle Claude".to_string(), EditorCommand::ToggleChat),
        (
            "View: installed plugins".to_string(),
            EditorCommand::ShowInstalled,
        ),
        (
            "View: plugin manager".to_string(),
            EditorCommand::ShowManager,
        ),
        ("View: files".to_string(), EditorCommand::ShowFiles),
        ("View: search".to_string(), EditorCommand::ShowSearch),
        ("Open folder".to_string(), EditorCommand::OpenFolder),
        ("Save file".to_string(), EditorCommand::SaveFile),
        ("Save all".to_string(), EditorCommand::SaveAll),
        ("Close tab".to_string(), EditorCommand::CloseTab),
        ("Next tab".to_string(), EditorCommand::NextTab),
        ("Previous tab".to_string(), EditorCommand::PrevTab),
        ("Balance splits".to_string(), EditorCommand::BalanceSplits),
        ("Find and replace".to_string(), EditorCommand::Find),
        ("Jump to word".to_string(), EditorCommand::JumpWord),
        ("Jump to line".to_string(), EditorCommand::JumpLine),
        ("Jump to char".to_string(), EditorCommand::JumpChar),
        (
            "Toggle rust-analyzer log".to_string(),
            EditorCommand::ToggleLspLog,
        ),
        ("New plugin".to_string(), EditorCommand::NewPlugin),
        ("Run or pause plugins".to_string(), EditorCommand::RunPause),
        ("Reset scene".to_string(), EditorCommand::ResetScene),
        ("Next theme".to_string(), EditorCommand::NextTheme),
        ("Help: keybindings".to_string(), EditorCommand::OpenHelp),
    ];
    for (id, label) in THEMES {
        items.push((
            format!("Theme: {label}"),
            EditorCommand::SetTheme(id.to_string()),
        ));
    }
    for plugin in state.plugins.get() {
        items.push((
            format!("Open: {}", plugin.name),
            EditorCommand::OpenBuffer {
                kind: PluginKind::Scene,
                id: plugin.id,
            },
        ));
    }
    for plugin in state.editor_plugins.get() {
        items.push((
            format!("Open: {} (editor)", plugin.name),
            EditorCommand::OpenBuffer {
                kind: PluginKind::Editor,
                id: plugin.id,
            },
        ));
    }
    for module in state.stdlib.get() {
        items.push((
            format!("Open: {} (built-in)", module.name),
            EditorCommand::OpenBuffer {
                kind: PluginKind::Builtin,
                id: module.name,
            },
        ));
    }
    items
}

/// Performs an editor command.
pub fn run(
    command: EditorCommand,
    state: EditorState,
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
) {
    match command {
        EditorCommand::SplitEditor { vertical } => state.split(vertical),
        EditorCommand::CloseSplit => state.close_focused(),
        EditorCommand::FocusOther => state.focus_next(),
        EditorCommand::TogglePreview => state.viewport_open.update(|open| *open = !*open),
        EditorCommand::ToggleConsole => state.console_open.update(|open| *open = !*open),
        EditorCommand::ToggleReference => state.reference_open.update(|open| *open = !*open),
        EditorCommand::ToggleChat => state.chat_open.update(|open| *open = !*open),
        EditorCommand::ShowInstalled => state.sidebar_view.set(SidebarView::Installed),
        EditorCommand::ShowManager => state.sidebar_view.set(SidebarView::Extensions),
        EditorCommand::ShowFiles => state.sidebar_view.set(SidebarView::Files),
        EditorCommand::ShowSearch => state.sidebar_view.set(SidebarView::Search),
        EditorCommand::OpenFolder => crate::fs::open_folder(),
        EditorCommand::SaveFile => {
            let buffer = state.focused_buffer();
            if buffer.kind == PluginKind::File
                && let Some(path) = buffer.id
            {
                let text = state.buffer_source(PluginKind::File, &Some(path.clone()));
                crate::fs::write_file(&path, text);
            }
        }
        EditorCommand::SaveAll => {
            for (path, text) in state
                .files
                .get_untracked()
                .into_iter()
                .filter_map(|file| file.dirty.then_some((file.path, file.text)))
            {
                crate::fs::write_file(&path, text);
            }
        }
        EditorCommand::CloseTab => state.close_focused_tab(),
        EditorCommand::NextTab => state.cycle_tab(1),
        EditorCommand::PrevTab => state.cycle_tab(-1),
        EditorCommand::FocusNext => state.focus_next(),
        EditorCommand::FocusPrev => state.focus_prev(),
        EditorCommand::BalanceSplits => state.balance_splits(),
        EditorCommand::Find => state.find_open.set(true),
        EditorCommand::JumpWord => crate::jump::start(state, crate::jump::JumpKind::Word),
        EditorCommand::JumpLine => crate::jump::start(state, crate::jump::JumpKind::Line),
        EditorCommand::JumpChar => crate::jump::start_char(state),
        EditorCommand::ToggleLspLog => state.lsp_log_open.update(|open| *open = !*open),
        EditorCommand::NewPlugin => {
            let plugin = plugins::new_plugin("Untitled");
            let id = plugin.id.clone();
            state.plugins.update(|plugins| plugins.push(plugin));
            state.open_in_focused(PluginKind::Scene, Some(id));
            if let Some(bridge) = bridge.get_value() {
                bridge::sync_plugins(&bridge, state);
            }
        }
        EditorCommand::RunPause => {
            let running = !state.running.get_untracked();
            state.running.set(running);
            if let Some(bridge) = bridge.get_value() {
                bridge::send(&bridge, &protocol::ClientMessage::SetRunning { running });
            }
        }
        EditorCommand::ResetScene => {
            if let Some(bridge) = bridge.get_value() {
                bridge::send(&bridge, &protocol::ClientMessage::ResetScene);
            }
        }
        EditorCommand::NextTheme => {
            let current = state.theme.get_untracked();
            let index = THEMES
                .iter()
                .position(|(id, _)| *id == current)
                .unwrap_or(0);
            let next = THEMES[(index + 1) % THEMES.len()].0;
            state.theme.set(next.to_string());
        }
        EditorCommand::OpenPalette => state.palette_open.set(true),
        EditorCommand::OpenHelp => state.help_open.set(true),
        EditorCommand::SetTheme(id) => state.theme.set(id),
        EditorCommand::OpenBuffer { kind, id } => state.open_in_focused(kind, Some(id)),
    }
}
