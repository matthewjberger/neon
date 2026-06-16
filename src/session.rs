//! Session persistence: the opened workspace folder and the files open in it,
//! saved to local storage and restored on launch, so neon reopens where you left
//! off. The plugin set persists separately (`plugins.rs`).

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::state::EditorState;

const KEY: &str = "neon.session";

#[derive(Default, Serialize, Deserialize)]
struct Session {
    root: Option<String>,
    files: Vec<String>,
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// Reopens the saved folder and files. Called once the filesystem socket opens,
/// so the requests are not dropped.
pub fn restore() {
    let Some(text) = storage().and_then(|storage| storage.get_item(KEY).ok().flatten()) else {
        return;
    };
    let Ok(session) = serde_json::from_str::<Session>(&text) else {
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
        root: state.workspace_root.get_untracked(),
        files: state
            .files
            .with_untracked(|files| files.iter().map(|file| file.path.clone()).collect()),
    };
    if let (Some(storage), Ok(text)) = (storage(), serde_json::to_string(&session)) {
        let _ = storage.set_item(KEY, &text);
    }
}
