//! Every floating overlay mounted over the shell: the LSP popups and menus, the
//! find bar, the which-key and jump overlays, the palette, prompts, the context
//! menu, multi-cursor carets, chat, and the loader. Each self-gates on its own
//! signal, so mounting them is a flat list; this is the one place that list
//! lives, so adding an overlay is a single edit here instead of in the shell.

pub mod chat;
pub mod context_menu;
pub mod find;
pub mod help;
pub mod jump_overlay;
pub mod loader;
pub mod lsp_menus;
pub mod multicursor;
pub mod palette;
pub mod popups;
pub mod prompt;
pub mod which_key;

use leptos::prelude::*;

use crate::bridge::Bridge;
use crate::components::overlays::chat::ChatPane;
use crate::components::overlays::context_menu::ContextMenuView;
use crate::components::overlays::find::FindBar;
use crate::components::overlays::help::Help;
use crate::components::overlays::jump_overlay::JumpOverlay;
use crate::components::overlays::loader::Loader;
use crate::components::overlays::lsp_menus::{CodeActionMenu, RenamePrompt, SymbolPicker};
use crate::components::overlays::multicursor::MultiCursorOverlay;
use crate::components::overlays::palette::Palette;
use crate::components::overlays::popups::{CompletionPopup, HoverCardView};
use crate::components::overlays::prompt::PromptView;
use crate::components::overlays::which_key::WhichKey;
use crate::components::panels::control_panel::ControlPanel;
use crate::components::panels::lsp_panel::{LspConsent, LspLog};
use crate::components::panels::problems::ProblemsPanel;
use crate::components::panels::undo_tree::UndoTree;
use crate::state::EditorState;

/// Mounts all overlays. The shell renders this once; the overlays decide their
/// own visibility from state.
#[component]
pub fn Overlays(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    state: EditorState,
) -> impl IntoView {
    view! {
        <FindBar state />
        <CompletionPopup state />
        <HoverCardView state />
        <JumpOverlay state />
        <WhichKey state />
        <LspConsent state />
        <LspLog state />
        <ProblemsPanel state />
        <UndoTree state />
        <MultiCursorOverlay state />
        <RenamePrompt state />
        <CodeActionMenu state />
        <SymbolPicker state />
        <ControlPanel bridge state />
        <ContextMenuView bridge state />
        <PromptView state />
        <Palette bridge state />
        <Help state />
        <ChatPane state />
        <Loader state />
    }
}
