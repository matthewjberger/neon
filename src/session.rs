//! Session persistence: the opened workspace folder and the files open in it,
//! saved to local storage and restored on launch, so neon reopens where you left
//! off. The plugin set persists separately (`plugins.rs`).

use std::cell::RefCell;

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::state::{EditorState, Pane};

const KEY: &str = "neon.session";

#[derive(Default, Serialize, Deserialize)]
struct Session {
    root: Option<String>,
    files: Vec<String>,
    #[serde(default)]
    panes: Vec<Pane>,
    #[serde(default)]
    focused: usize,
}

thread_local! {
    static PENDING: RefCell<Option<Session>> = const { RefCell::new(None) };
}

/// Reads the saved session into memory at startup, before the save effect runs
/// and overwrites it with the empty initial state.
pub fn capture() {
    let session = crate::storage::get_json::<Session>(KEY);
    PENDING.with(|pending| *pending.borrow_mut() = session);
}

/// Restores the saved tile layout into the panes, before the filesystem socket
/// opens. File tiles fill in once their content loads. Skipped when nothing was
/// saved, so a first run keeps the default layout.
pub fn restore_layout(state: EditorState) {
    PENDING.with(|pending| {
        if let Some(session) = pending.borrow().as_ref()
            && !session.panes.is_empty()
        {
            state.panes.set(session.panes.clone());
            state.focused_key.set(session.focused);
        }
    });
}

/// Reopens the captured folder and files. Called once the filesystem socket
/// opens, so the requests are not dropped.
pub fn restore() {
    let Some(session) = PENDING.with(|pending| pending.borrow_mut().take()) else {
        return;
    };
    if let Some(root) = session.root {
        crate::fs::open_root(&root);
    }
    for path in session.files {
        crate::fs::read_file(&path);
    }
}

/// Saves the current workspace and open files.
pub fn save(state: EditorState) {
    let session = Session {
        root: state.explorer.root.get_untracked(),
        files: state
            .files
            .with_untracked(|files| files.iter().map(|file| file.path.clone()).collect()),
        panes: state.panes.get_untracked(),
        focused: state.focused_key.get_untracked(),
    };
    crate::storage::set_json(KEY, &session);
}
