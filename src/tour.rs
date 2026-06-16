//! The interactive tour: a guided sequence that teaches the core keys by having
//! you do each one. Most steps name the command they expect, so the tour
//! advances when you actually perform the action, watched through the command
//! registry. Steps without a command advance with the Next button.

use leptos::prelude::*;

use crate::state::EditorState;

/// One tour step: a title, the instruction, and the command id that advances it.
pub struct Step {
    pub title: &'static str,
    pub body: &'static str,
    pub expected: Option<&'static str>,
}

pub const STEPS: &[Step] = &[
    Step {
        title: "Welcome to Neon",
        body: "A quick tour of the keys. Do each step, or press Next. You can press Skip anytime.",
        expected: None,
    },
    Step {
        title: "Modes",
        body: "Neon is modal. Press i to insert text, then Esc to return to normal mode.",
        expected: None,
    },
    Step {
        title: "Move around",
        body: "In normal mode, h j k l move left, down, up, and right. The arrow keys work too.",
        expected: None,
    },
    Step {
        title: "The leader",
        body: "Press SPC in normal mode to open the which-key menu. Every command hangs off it.",
        expected: None,
    },
    Step {
        title: "Command palette",
        body: "Press SPC SPC to open the palette and run any command by name.",
        expected: Some("open-palette"),
    },
    Step {
        title: "Open a folder",
        body: "Press SPC f f to open a project folder in the file tree.",
        expected: Some("open-folder"),
    },
    Step {
        title: "Search the project",
        body: "Press SPC / to search the whole project. The query is a smart-case regex.",
        expected: Some("show-search"),
    },
    Step {
        title: "Jump to a word",
        body: "Press SPC j w, then type the label on the word you want. The caret jumps there.",
        expected: Some("jump-word"),
    },
    Step {
        title: "Go to definition",
        body: "On a Rust symbol, press gd (or SPC g g) to jump to where it is defined.",
        expected: Some("go-to-definition"),
    },
    Step {
        title: "Save",
        body: "Press SPC f s to save the current file.",
        expected: Some("save-file"),
    },
    Step {
        title: "The full reference",
        body: "Press SPC ? anytime for the complete keybinding reference and every command.",
        expected: Some("open-help"),
    },
    Step {
        title: "You are ready",
        body: "That is the core. Walk the leader menus to find the rest. Press Finish to close.",
        expected: None,
    },
];

/// Starts the tour at the first step.
pub fn start(state: EditorState) {
    state.tour.set(Some(0));
}

/// Closes the tour.
pub fn close(state: EditorState) {
    state.tour.set(None);
}

/// Advances to the next step, closing the tour after the last.
pub fn next(state: EditorState) {
    state.tour.update(|step| {
        if let Some(index) = *step {
            *step = if index + 1 < STEPS.len() {
                Some(index + 1)
            } else {
                None
            };
        }
    });
}

/// Advances the tour if the command just run is the one the current step waits
/// for. Called for every command the registry dispatches.
pub fn observe(state: EditorState, id: &str) {
    let waiting_for = state
        .tour
        .get_untracked()
        .and_then(|index| STEPS.get(index))
        .and_then(|step| step.expected);
    if waiting_for == Some(id) {
        next(state);
    }
}
