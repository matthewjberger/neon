//! Every floating overlay mounted over the shell: the LSP popups and menus, the
//! find bar, the which-key and jump overlays, the palette, prompts, the context
//! menu, multi-cursor carets, chat, and the loader. Each self-gates on its own
//! signal, so mounting them is a flat list; this is the one place that list
//! lives, so adding an overlay is a single edit here instead of in the shell.

use leptos::prelude::*;

use crate::bridge::Bridge;
use crate::components::chat::ChatPane;
use crate::components::context_menu::ContextMenuView;
use crate::components::control_panel::ControlPanel;
use crate::components::find::FindBar;
use crate::components::help::Help;
use crate::components::jump_overlay::JumpOverlay;
use crate::components::loader::Loader;
use crate::components::lsp_menus::{CodeActionMenu, RenamePrompt, SymbolPicker};
use crate::components::lsp_panel::{LspConsent, LspLog};
use crate::components::multicursor::MultiCursorOverlay;
use crate::components::palette::Palette;
use crate::components::popups::{CompletionPopup, HoverCardView};
use crate::components::problems::ProblemsPanel;
use crate::components::prompt::PromptView;
use crate::components::which_key::WhichKey;
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
