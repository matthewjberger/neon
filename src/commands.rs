//! The editor operations registry. Each command is a named editor action the
//! command palette lists and an editor plugin can invoke by id (the way vim's
//! `:` and Spacemacs' SPC menus drive the editor). One place defines what the
//! editor can do.

use leptos::prelude::*;

use crate::bridge::{self, Bridge};
use crate::plugins;
use crate::state::{EditorState, PluginKind, Prompt, PromptAction, SidebarView};
use crate::theme::THEMES;

/// One editor operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorCommand {
    SplitEditor { vertical: bool },
    CloseSplit,
    FocusOther,
    NewWindow,
    TogglePreview,
    ToggleConsole,
    ToggleReference,
    ToggleControlPanel,
    ToggleChat,
    ShowInstalled,
    ShowManager,
    ShowFiles,
    ShowSearch,
    OpenFolder,
    NewFile,
    RenameEntry,
    DeleteEntry,
    SaveFile,
    SaveAll,
    CloseTab,
    NextTab,
    PrevTab,
    FocusNext,
    FocusPrev,
    BalanceSplits,
    Undo,
    Redo,
    Find,
    JumpWord,
    JumpLine,
    JumpChar,
    GoToDefinition,
    GoToTypeDefinition,
    GoToImplementation,
    FindReferences,
    JumpSymbol,
    WorkspaceSymbols,
    Hover,
    SignatureHelp,
    Rename,
    CodeAction,
    FormatDocument,
    NextError,
    PrevError,
    ToggleProblems,
    ToggleLspLog,
    CargoCheck,
    CargoBuild,
    CargoTest,
    CargoRun,
    Interrupt,
    ToggleTerminal,
    AddCursorBelow,
    AddCursorAbove,
    ClearCursors,
    NewPlugin,
    RunPause,
    ResetScene,
    NextTheme,
    ToggleFormatOnSave,
    OpenPalette,
    OpenHelp,
    Tour,
    SetTheme(String),
    OpenBuffer { kind: PluginKind, id: String },
}

/// Every static editor command in one place: its plugin-facing id, its palette
/// label (`None` when the command is reachable by id and the menus but not
/// listed in the palette), and the command itself. The id lookup and the
/// palette both derive from this, so a new command is declared once.
fn static_commands() -> Vec<(&'static str, Option<&'static str>, EditorCommand)> {
    use EditorCommand::*;
    vec![
        (
            "split-right",
            Some("Split right"),
            SplitEditor { vertical: true },
        ),
        (
            "split-below",
            Some("Split below"),
            SplitEditor { vertical: false },
        ),
        ("close-split", Some("Close split"), CloseSplit),
        ("focus-other", Some("Focus other pane"), FocusOther),
        ("new-window", Some("New window"), NewWindow),
        ("toggle-preview", Some("Toggle 3D preview"), TogglePreview),
        ("toggle-console", Some("Toggle console"), ToggleConsole),
        (
            "toggle-reference",
            Some("Toggle reference"),
            ToggleReference,
        ),
        (
            "toggle-control-panel",
            Some("Toggle control panel"),
            ToggleControlPanel,
        ),
        ("toggle-chat", Some("Toggle Claude"), ToggleChat),
        (
            "show-installed",
            Some("View: installed plugins"),
            ShowInstalled,
        ),
        ("show-manager", Some("View: plugin manager"), ShowManager),
        ("show-files", Some("View: files"), ShowFiles),
        ("show-search", Some("View: search"), ShowSearch),
        ("open-folder", Some("Open folder"), OpenFolder),
        ("new-file", Some("New file"), NewFile),
        ("save-file", Some("Save file"), SaveFile),
        ("save-all", Some("Save all"), SaveAll),
        ("close-tab", Some("Close tab"), CloseTab),
        ("next-tab", Some("Next tab"), NextTab),
        ("prev-tab", Some("Previous tab"), PrevTab),
        ("balance-splits", Some("Balance splits"), BalanceSplits),
        ("undo", Some("Undo"), Undo),
        ("redo", Some("Redo"), Redo),
        ("find", Some("Find and replace"), Find),
        ("jump-word", Some("Jump to word"), JumpWord),
        ("jump-line", Some("Jump to line"), JumpLine),
        ("jump-char", Some("Jump to char"), JumpChar),
        ("go-to-definition", Some("Go to definition"), GoToDefinition),
        (
            "go-to-type-definition",
            Some("Go to type definition"),
            GoToTypeDefinition,
        ),
        (
            "go-to-implementation",
            Some("Go to implementation"),
            GoToImplementation,
        ),
        ("find-references", Some("Find references"), FindReferences),
        ("jump-symbol", Some("Jump to symbol"), JumpSymbol),
        (
            "workspace-symbols",
            Some("Search workspace symbols"),
            WorkspaceSymbols,
        ),
        ("hover", Some("Show hover"), Hover),
        ("signature-help", Some("Signature help"), SignatureHelp),
        ("rename-symbol", Some("Rename symbol"), Rename),
        ("code-action", Some("Code action"), CodeAction),
        ("format-document", Some("Format document"), FormatDocument),
        ("next-error", Some("Next error"), NextError),
        ("prev-error", Some("Previous error"), PrevError),
        ("toggle-problems", Some("Toggle problems"), ToggleProblems),
        ("cargo-check", Some("Cargo check"), CargoCheck),
        ("cargo-build", Some("Cargo build"), CargoBuild),
        ("cargo-test", Some("Cargo test"), CargoTest),
        ("cargo-run", Some("Cargo run"), CargoRun),
        ("interrupt", Some("Interrupt terminal"), Interrupt),
        ("toggle-terminal", Some("Toggle terminal"), ToggleTerminal),
        ("add-cursor-below", Some("Add cursor below"), AddCursorBelow),
        ("add-cursor-above", Some("Add cursor above"), AddCursorAbove),
        ("clear-cursors", Some("Clear extra cursors"), ClearCursors),
        (
            "toggle-lsp-log",
            Some("Toggle rust-analyzer log"),
            ToggleLspLog,
        ),
        ("new-plugin", Some("New plugin"), NewPlugin),
        ("run-pause", Some("Run or pause plugins"), RunPause),
        ("reset-scene", Some("Reset scene"), ResetScene),
        ("next-theme", Some("Next theme"), NextTheme),
        (
            "toggle-format-on-save",
            Some("Toggle format on save"),
            ToggleFormatOnSave,
        ),
        ("open-help", Some("Help: keybindings"), OpenHelp),
        ("tour", Some("Tour: learn the keys"), Tour),
        ("rename-entry", None, RenameEntry),
        ("delete-entry", None, DeleteEntry),
        ("focus-next", None, FocusNext),
        ("focus-prev", None, FocusPrev),
        ("open-palette", None, OpenPalette),
    ]
}

/// The id an editor plugin uses to invoke a static command.
pub fn command_from_id(id: &str) -> Option<EditorCommand> {
    static_commands()
        .into_iter()
        .find(|(command_id, _, _)| *command_id == id)
        .map(|(_, _, command)| command)
}

/// Every command the palette offers: the static operations that carry a label,
/// a theme per theme, and an open command per installed plugin and built-in
/// module.
pub fn palette_items(state: EditorState) -> Vec<(String, EditorCommand)> {
    let mut items: Vec<(String, EditorCommand)> = static_commands()
        .into_iter()
        .filter_map(|(_, label, command)| label.map(|label| (label.to_string(), command)))
        .collect();
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

/// The parent directory of a path, splitting on either separator.
fn parent_of(path: &str) -> String {
    match path.rfind(['\\', '/']) {
        Some(index) => path[..index].to_string(),
        None => String::new(),
    }
}

/// A short human label for an editor command, for the api log.
fn command_label(command: &EditorCommand) -> String {
    match command {
        EditorCommand::SetTheme(id) => format!("SetTheme({id})"),
        EditorCommand::OpenBuffer { kind, id } => format!("OpenBuffer({kind:?}, {id:?})"),
        other => format!("{other:?}"),
    }
}

/// Performs an editor command.
pub fn run(
    command: EditorCommand,
    state: EditorState,
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
) {
    state.log_api(
        protocol::LogKind::Command,
        "editor",
        command_label(&command),
    );
    match command {
        EditorCommand::SplitEditor { vertical } => state.split(vertical),
        EditorCommand::CloseSplit => state.close_focused(),
        EditorCommand::FocusOther => state.focus_next(),
        EditorCommand::NewWindow => crate::network::request_spawn_window(),
        EditorCommand::TogglePreview => state.panels.viewport.update(|open| *open = !*open),
        EditorCommand::ToggleConsole => state.panels.console.update(|open| *open = !*open),
        EditorCommand::ToggleReference => state.panels.reference.update(|open| *open = !*open),
        EditorCommand::ToggleControlPanel => {
            state.panels.control_panel.update(|open| *open = !*open)
        }
        EditorCommand::ToggleChat => state.panels.chat.update(|open| *open = !*open),
        EditorCommand::ShowInstalled => state.sidebar_view.set(SidebarView::Installed),
        EditorCommand::ShowManager => state.sidebar_view.set(SidebarView::Extensions),
        EditorCommand::ShowFiles => state.sidebar_view.set(SidebarView::Files),
        EditorCommand::ShowSearch => state.sidebar_view.set(SidebarView::Search),
        EditorCommand::OpenFolder => crate::fs::open_folder(),
        EditorCommand::NewFile => {
            let dir = match state.editing.context_target.get_untracked() {
                Some((path, true)) => path,
                Some((path, false)) => parent_of(&path),
                None => state.explorer.root.get_untracked().unwrap_or_default(),
            };
            if !dir.is_empty() {
                state.editing.prompt.set(Some(Prompt {
                    title: "New file".to_string(),
                    value: String::new(),
                    action: PromptAction::CreateFile { dir },
                }));
            }
        }
        EditorCommand::RenameEntry => {
            if let Some((from, _)) = state.editing.context_target.get_untracked() {
                let value = crate::state::basename(&from).to_string();
                state.editing.prompt.set(Some(Prompt {
                    title: "Rename".to_string(),
                    value,
                    action: PromptAction::RenameEntry { from },
                }));
            }
        }
        EditorCommand::DeleteEntry => {
            if let Some((path, _)) = state.editing.context_target.get_untracked() {
                let title = format!("Delete {}? Enter to confirm", crate::state::basename(&path));
                state.editing.prompt.set(Some(Prompt {
                    title,
                    value: String::new(),
                    action: PromptAction::DeleteEntry { path },
                }));
            }
        }
        EditorCommand::SaveFile => {
            let buffer = state.focused_buffer();
            if buffer.kind == PluginKind::File
                && let Some(path) = buffer.id
            {
                let formatted = state.lsp.format_on_save.get_untracked()
                    && crate::state::language_for_path(&path) == "rust"
                    && crate::lsp::format_and_save(state, &path);
                if !formatted {
                    let text = state.buffer_source(PluginKind::File, &Some(path.clone()));
                    crate::fs::write_file(&path, text);
                }
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
        EditorCommand::Undo => crate::undo::undo(state),
        EditorCommand::Redo => crate::undo::redo(state),
        EditorCommand::Find => state.editing.find_open.set(true),
        EditorCommand::JumpWord => crate::jump::start(state, crate::jump::JumpKind::Word),
        EditorCommand::JumpLine => crate::jump::start(state, crate::jump::JumpKind::Line),
        EditorCommand::JumpChar => crate::jump::start_char(state),
        EditorCommand::GoToDefinition => {
            crate::lsp::request_locations(state, "textDocument/definition")
        }
        EditorCommand::GoToTypeDefinition => {
            crate::lsp::request_locations(state, "textDocument/typeDefinition")
        }
        EditorCommand::GoToImplementation => {
            crate::lsp::request_locations(state, "textDocument/implementation")
        }
        EditorCommand::FindReferences => crate::lsp::request_references(state),
        EditorCommand::JumpSymbol => crate::lsp::request_symbols(state),
        EditorCommand::WorkspaceSymbols => crate::lsp::request_workspace_symbols(),
        EditorCommand::Hover => crate::lsp::request_hover_at_caret(state),
        EditorCommand::SignatureHelp => crate::lsp::request_signature_help(state),
        EditorCommand::Rename => crate::lsp::start_rename(state),
        EditorCommand::CodeAction => crate::lsp::request_code_actions(state),
        EditorCommand::FormatDocument => crate::lsp::format_document(state),
        EditorCommand::NextError => crate::lsp::goto_diagnostic(state, true),
        EditorCommand::PrevError => crate::lsp::goto_diagnostic(state, false),
        EditorCommand::ToggleProblems => state.lsp.problems_open.update(|open| *open = !*open),
        EditorCommand::ToggleLspLog => state.lsp.log_open.update(|open| *open = !*open),
        EditorCommand::CargoCheck => crate::terminal::run(state, "cargo check"),
        EditorCommand::CargoBuild => crate::terminal::run(state, "cargo build"),
        EditorCommand::CargoTest => crate::terminal::run(state, "cargo test"),
        EditorCommand::CargoRun => crate::terminal::run(state, "cargo run"),
        EditorCommand::Interrupt => crate::terminal::interrupt(),
        EditorCommand::ToggleTerminal => state.terminal.open.update(|open| *open = !*open),
        EditorCommand::AddCursorBelow => crate::multicursor::add_below(state),
        EditorCommand::AddCursorAbove => crate::multicursor::add_above(state),
        EditorCommand::ClearCursors => crate::multicursor::clear(state),
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
        EditorCommand::ToggleFormatOnSave => state
            .lsp
            .format_on_save
            .update(|enabled| *enabled = !*enabled),
        EditorCommand::OpenPalette => state.editing.palette_open.set(true),
        EditorCommand::OpenHelp => state.panels.help.set(true),
        EditorCommand::Tour => crate::tour::start(state),
        EditorCommand::SetTheme(id) => state.theme.set(id),
        EditorCommand::OpenBuffer { kind, id } => state.open_in_focused(kind, Some(id)),
    }
}
