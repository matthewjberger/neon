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
    CancelTask,
    ToggleTasks,
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
        "toggle-control-panel" => EditorCommand::ToggleControlPanel,
        "toggle-chat" => EditorCommand::ToggleChat,
        "show-installed" => EditorCommand::ShowInstalled,
        "show-manager" => EditorCommand::ShowManager,
        "show-files" => EditorCommand::ShowFiles,
        "show-search" => EditorCommand::ShowSearch,
        "open-folder" => EditorCommand::OpenFolder,
        "new-file" => EditorCommand::NewFile,
        "rename-entry" => EditorCommand::RenameEntry,
        "delete-entry" => EditorCommand::DeleteEntry,
        "save-file" => EditorCommand::SaveFile,
        "save-all" => EditorCommand::SaveAll,
        "close-tab" => EditorCommand::CloseTab,
        "next-tab" => EditorCommand::NextTab,
        "prev-tab" => EditorCommand::PrevTab,
        "focus-next" => EditorCommand::FocusNext,
        "focus-prev" => EditorCommand::FocusPrev,
        "balance-splits" => EditorCommand::BalanceSplits,
        "undo" => EditorCommand::Undo,
        "redo" => EditorCommand::Redo,
        "find" => EditorCommand::Find,
        "jump-word" => EditorCommand::JumpWord,
        "jump-line" => EditorCommand::JumpLine,
        "jump-char" => EditorCommand::JumpChar,
        "go-to-definition" => EditorCommand::GoToDefinition,
        "go-to-type-definition" => EditorCommand::GoToTypeDefinition,
        "go-to-implementation" => EditorCommand::GoToImplementation,
        "find-references" => EditorCommand::FindReferences,
        "jump-symbol" => EditorCommand::JumpSymbol,
        "workspace-symbols" => EditorCommand::WorkspaceSymbols,
        "hover" => EditorCommand::Hover,
        "signature-help" => EditorCommand::SignatureHelp,
        "rename-symbol" => EditorCommand::Rename,
        "code-action" => EditorCommand::CodeAction,
        "format-document" => EditorCommand::FormatDocument,
        "next-error" => EditorCommand::NextError,
        "prev-error" => EditorCommand::PrevError,
        "toggle-problems" => EditorCommand::ToggleProblems,
        "toggle-lsp-log" => EditorCommand::ToggleLspLog,
        "cargo-check" => EditorCommand::CargoCheck,
        "cargo-build" => EditorCommand::CargoBuild,
        "cargo-test" => EditorCommand::CargoTest,
        "cargo-run" => EditorCommand::CargoRun,
        "cancel-task" => EditorCommand::CancelTask,
        "toggle-tasks" => EditorCommand::ToggleTasks,
        "add-cursor-below" => EditorCommand::AddCursorBelow,
        "add-cursor-above" => EditorCommand::AddCursorAbove,
        "clear-cursors" => EditorCommand::ClearCursors,
        "new-plugin" => EditorCommand::NewPlugin,
        "run-pause" => EditorCommand::RunPause,
        "reset-scene" => EditorCommand::ResetScene,
        "next-theme" => EditorCommand::NextTheme,
        "toggle-format-on-save" => EditorCommand::ToggleFormatOnSave,
        "open-palette" => EditorCommand::OpenPalette,
        "open-help" => EditorCommand::OpenHelp,
        "tour" => EditorCommand::Tour,
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
        (
            "Toggle control panel".to_string(),
            EditorCommand::ToggleControlPanel,
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
        ("New file".to_string(), EditorCommand::NewFile),
        ("Save file".to_string(), EditorCommand::SaveFile),
        ("Save all".to_string(), EditorCommand::SaveAll),
        ("Close tab".to_string(), EditorCommand::CloseTab),
        ("Next tab".to_string(), EditorCommand::NextTab),
        ("Previous tab".to_string(), EditorCommand::PrevTab),
        ("Balance splits".to_string(), EditorCommand::BalanceSplits),
        ("Undo".to_string(), EditorCommand::Undo),
        ("Redo".to_string(), EditorCommand::Redo),
        ("Find and replace".to_string(), EditorCommand::Find),
        ("Jump to word".to_string(), EditorCommand::JumpWord),
        ("Jump to line".to_string(), EditorCommand::JumpLine),
        ("Jump to char".to_string(), EditorCommand::JumpChar),
        (
            "Go to definition".to_string(),
            EditorCommand::GoToDefinition,
        ),
        (
            "Go to type definition".to_string(),
            EditorCommand::GoToTypeDefinition,
        ),
        (
            "Go to implementation".to_string(),
            EditorCommand::GoToImplementation,
        ),
        ("Find references".to_string(), EditorCommand::FindReferences),
        ("Jump to symbol".to_string(), EditorCommand::JumpSymbol),
        (
            "Search workspace symbols".to_string(),
            EditorCommand::WorkspaceSymbols,
        ),
        ("Show hover".to_string(), EditorCommand::Hover),
        ("Signature help".to_string(), EditorCommand::SignatureHelp),
        ("Rename symbol".to_string(), EditorCommand::Rename),
        ("Code action".to_string(), EditorCommand::CodeAction),
        ("Format document".to_string(), EditorCommand::FormatDocument),
        ("Next error".to_string(), EditorCommand::NextError),
        ("Previous error".to_string(), EditorCommand::PrevError),
        ("Toggle problems".to_string(), EditorCommand::ToggleProblems),
        ("Cargo check".to_string(), EditorCommand::CargoCheck),
        ("Cargo build".to_string(), EditorCommand::CargoBuild),
        ("Cargo test".to_string(), EditorCommand::CargoTest),
        ("Cargo run".to_string(), EditorCommand::CargoRun),
        ("Cancel task".to_string(), EditorCommand::CancelTask),
        ("Toggle terminal".to_string(), EditorCommand::ToggleTasks),
        (
            "Add cursor below".to_string(),
            EditorCommand::AddCursorBelow,
        ),
        (
            "Add cursor above".to_string(),
            EditorCommand::AddCursorAbove,
        ),
        (
            "Clear extra cursors".to_string(),
            EditorCommand::ClearCursors,
        ),
        (
            "Toggle rust-analyzer log".to_string(),
            EditorCommand::ToggleLspLog,
        ),
        ("New plugin".to_string(), EditorCommand::NewPlugin),
        ("Run or pause plugins".to_string(), EditorCommand::RunPause),
        ("Reset scene".to_string(), EditorCommand::ResetScene),
        ("Next theme".to_string(), EditorCommand::NextTheme),
        (
            "Toggle format on save".to_string(),
            EditorCommand::ToggleFormatOnSave,
        ),
        ("Help: keybindings".to_string(), EditorCommand::OpenHelp),
        ("Tour: learn the keys".to_string(), EditorCommand::Tour),
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
        EditorCommand::TogglePreview => state.viewport_open.update(|open| *open = !*open),
        EditorCommand::ToggleConsole => state.console_open.update(|open| *open = !*open),
        EditorCommand::ToggleReference => state.reference_open.update(|open| *open = !*open),
        EditorCommand::ToggleControlPanel => state.control_panel_open.update(|open| *open = !*open),
        EditorCommand::ToggleChat => state.chat_open.update(|open| *open = !*open),
        EditorCommand::ShowInstalled => state.sidebar_view.set(SidebarView::Installed),
        EditorCommand::ShowManager => state.sidebar_view.set(SidebarView::Extensions),
        EditorCommand::ShowFiles => state.sidebar_view.set(SidebarView::Files),
        EditorCommand::ShowSearch => state.sidebar_view.set(SidebarView::Search),
        EditorCommand::OpenFolder => crate::fs::open_folder(),
        EditorCommand::NewFile => {
            let dir = match state.context_target.get_untracked() {
                Some((path, true)) => path,
                Some((path, false)) => parent_of(&path),
                None => state.workspace_root.get_untracked().unwrap_or_default(),
            };
            if !dir.is_empty() {
                state.prompt.set(Some(Prompt {
                    title: "New file".to_string(),
                    value: String::new(),
                    action: PromptAction::CreateFile { dir },
                }));
            }
        }
        EditorCommand::RenameEntry => {
            if let Some((from, _)) = state.context_target.get_untracked() {
                let value = crate::state::basename(&from).to_string();
                state.prompt.set(Some(Prompt {
                    title: "Rename".to_string(),
                    value,
                    action: PromptAction::RenameEntry { from },
                }));
            }
        }
        EditorCommand::DeleteEntry => {
            if let Some((path, _)) = state.context_target.get_untracked() {
                let title = format!("Delete {}? Enter to confirm", crate::state::basename(&path));
                state.prompt.set(Some(Prompt {
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
                let formatted = state.format_on_save.get_untracked()
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
        EditorCommand::Find => state.find_open.set(true),
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
        EditorCommand::ToggleProblems => state.problems_open.update(|open| *open = !*open),
        EditorCommand::ToggleLspLog => state.lsp_log_open.update(|open| *open = !*open),
        EditorCommand::CargoCheck => crate::tasks::run(state, "cargo", &["check"]),
        EditorCommand::CargoBuild => crate::tasks::run(state, "cargo", &["build"]),
        EditorCommand::CargoTest => crate::tasks::run(state, "cargo", &["test"]),
        EditorCommand::CargoRun => crate::tasks::run(state, "cargo", &["run"]),
        EditorCommand::CancelTask => crate::tasks::cancel(state),
        EditorCommand::ToggleTasks => state.task_open.update(|open| *open = !*open),
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
        EditorCommand::ToggleFormatOnSave => {
            state.format_on_save.update(|enabled| *enabled = !*enabled)
        }
        EditorCommand::OpenPalette => state.palette_open.set(true),
        EditorCommand::OpenHelp => state.help_open.set(true),
        EditorCommand::Tour => crate::tour::start(state),
        EditorCommand::SetTheme(id) => state.theme.set(id),
        EditorCommand::OpenBuffer { kind, id } => state.open_in_focused(kind, Some(id)),
    }
}
