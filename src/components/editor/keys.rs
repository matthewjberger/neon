//! The editor key router: a keydown becomes completion navigation, a
//! multi-cursor edit, the built-in newline and tab handling, or an editor-
//! plugin dispatch. A free function over plain data.

use leptos::html;
use leptos::prelude::*;

use crate::bridge::Bridge;
use crate::editor_plugins;
use crate::lang::Lang;
use crate::state::{EditorState, kind_readonly};

use super::{commit, current_buffer, inserts_newline, newline_indent};

/// The handles a keystroke needs to read and edit the focused buffer, bundled so
/// the router is a free function over plain data rather than a closure capturing
/// the component scope.
pub(super) struct KeyContext {
    pub(super) event: web_sys::KeyboardEvent,
    pub(super) state: EditorState,
    pub(super) bridge: StoredValue<Option<Bridge>, LocalStorage>,
    pub(super) lang: StoredValue<Option<Lang>, LocalStorage>,
    pub(super) pane_key: usize,
    pub(super) textarea: NodeRef<html::Textarea>,
    pub(super) debounce: StoredValue<Option<i32>>,
    pub(super) request_id: StoredValue<u32>,
}

/// Routes one keydown: the completion popup first, then multi-cursor, then the
/// built-in newline and tab handling, then the editor plugins.
pub(super) fn handle_keydown(ctx: KeyContext) {
    let KeyContext {
        event,
        state,
        bridge,
        lang,
        pane_key,
        textarea,
        debounce,
        request_id,
    } = ctx;
    // A lone modifier press (e.g. Shift on the way to a `?`) is never a
    // keystroke on its own; dispatching it would reset a pending leader
    // sequence and dismiss the which-key menu. Plugins read modifiers as flags
    // on a real key, so drop the standalone press here.
    if matches!(
        event.key().as_str(),
        "Shift" | "Control" | "Alt" | "Meta" | "CapsLock" | "AltGraph"
    ) {
        return;
    }
    if state.editing.jump.get_untracked().is_some() {
        return;
    }
    if state.lsp.completion.get_untracked().is_some() {
        let len = state
            .lsp
            .completion
            .with_untracked(|menu| menu.as_ref().map(|menu| menu.items.len()).unwrap_or(0))
            .max(1);
        match event.key().as_str() {
            "ArrowDown" => {
                event.prevent_default();
                state
                    .lsp
                    .completion_index
                    .update(|index| *index = (*index + 1) % len);
                return;
            }
            "ArrowUp" => {
                event.prevent_default();
                state
                    .lsp
                    .completion_index
                    .update(|index| *index = (*index + len - 1) % len);
                return;
            }
            "Enter" | "Tab" => {
                event.prevent_default();
                crate::lsp::accept_completion(state, state.lsp.completion_index.get_untracked());
                return;
            }
            "Escape" => {
                event.prevent_default();
                state.lsp.completion.set(None);
                return;
            }
            _ => {}
        }
    }
    let (id, kind) = current_buffer(state, pane_key);
    if kind_readonly(kind) {
        return;
    }
    if crate::multicursor::active(state) {
        match event.key().as_str() {
            "Escape" => {
                event.prevent_default();
                crate::multicursor::clear(state);
                return;
            }
            "Backspace" => {
                event.prevent_default();
                crate::multicursor::delete_back(state);
                return;
            }
            "Delete" => {
                event.prevent_default();
                crate::multicursor::delete_forward(state);
                return;
            }
            "Enter" => {
                event.prevent_default();
                crate::multicursor::insert(state, "\n");
                return;
            }
            "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | "Home" | "End" | "PageUp"
            | "PageDown" => {
                if !event.ctrl_key() && !event.alt_key() && !event.meta_key() {
                    crate::multicursor::clear(state);
                }
            }
            key => {
                if !event.ctrl_key()
                    && !event.alt_key()
                    && !event.meta_key()
                    && key.chars().count() == 1
                {
                    event.prevent_default();
                    crate::multicursor::insert(state, key);
                    return;
                }
            }
        }
    }
    if event.key() == "Enter" && inserts_newline(state) {
        if let Some(element) = textarea.get() {
            event.prevent_default();
            let caret = element.selection_start().ok().flatten().unwrap_or(0);
            let text = newline_indent(&element.value(), caret);
            editor_plugins::insert_text(state, id, kind, &element, &text);
            commit(bridge, lang, state, pane_key, debounce, request_id);
        }
        return;
    }
    if event.key() == "Tab" {
        event.prevent_default();
        if let Some(element) = textarea.get() {
            editor_plugins::insert_text(state, id, kind, &element, "    ");
            commit(bridge, lang, state, pane_key, debounce, request_id);
        }
        return;
    }
    if !editor_plugins::any_enabled(state) {
        return;
    }
    let Some(element) = textarea.get() else {
        return;
    };
    let outcome = editor_plugins::handle_key(
        state,
        id,
        kind,
        &element,
        &editor_plugins::KeyEvent {
            key: event.key(),
            ctrl: event.ctrl_key(),
            shift: event.shift_key(),
            alt: event.alt_key(),
        },
    );
    if outcome.consumed {
        event.prevent_default();
    }
    if outcome.changed {
        commit(bridge, lang, state, pane_key, debounce, request_id);
    }
}
