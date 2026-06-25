//! The page side of the filesystem bridge. Connects to the desktop filesystem
//! relay, sends `FsRequest`s, and applies each `FsResponse` to the state: the
//! opened folder and tree, lazily loaded directories, opened file buffers, and
//! save acknowledgements. The socket lives in a thread-local so the tree and the
//! commands can send without threading a handle through every component.

use std::cell::RefCell;

use leptos::prelude::*;
use protocol::{DirEntry, FsRequest, FsResponse, LogKind};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

use crate::state::{EditorState, FileBuffer, PluginKind, TileContent, TreeNode};

const FS_URL: &str = "ws://127.0.0.1:8792";
const RECONNECT_MS: i32 = 1000;

thread_local! {
    static SOCKET: RefCell<Option<WebSocket>> = const { RefCell::new(None) };
    static RESTORED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Opens the filesystem relay and keeps it open, reconnecting on drop.
pub fn start(state: EditorState) {
    connect(state);
}

fn connect(state: EditorState) {
    let Ok(websocket) = WebSocket::new(FS_URL) else {
        schedule_reconnect(state);
        return;
    };
    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        if let Some(text) = event.data().as_string()
            && let Ok(response) = serde_json::from_str::<FsResponse>(&text)
        {
            dispatch(state, response);
        }
    });
    websocket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let onopen = Closure::<dyn FnMut()>::new(move || {
        if !RESTORED.with(std::cell::Cell::get) {
            RESTORED.with(|flag| flag.set(true));
            crate::session::restore();
        }
    });
    websocket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let onclose = Closure::<dyn FnMut()>::new(move || schedule_reconnect(state));
    websocket.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();

    SOCKET.with(|slot| *slot.borrow_mut() = Some(websocket));
}

fn schedule_reconnect(state: EditorState) {
    SOCKET.with(|slot| *slot.borrow_mut() = None);
    let Some(window) = web_sys::window() else {
        return;
    };
    let callback = Closure::<dyn FnMut()>::new(move || connect(state));
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        RECONNECT_MS,
    );
    callback.forget();
}

fn send(request: &FsRequest) {
    SOCKET.with(|slot| {
        if let Some(websocket) = slot.borrow().as_ref()
            && websocket.ready_state() == WebSocket::OPEN
            && let Ok(text) = serde_json::to_string(request)
        {
            let _ = websocket.send_with_str(&text);
        }
    });
}

/// Opens the native folder picker.
pub fn open_folder() {
    send(&FsRequest::OpenFolder { request_id: 0 });
}

/// Opens a known folder by path, no dialog, to restore a session.
pub fn open_root(path: &str) {
    send(&FsRequest::OpenRoot {
        request_id: 0,
        path: path.to_string(),
    });
}

/// Lists one directory's children, to expand a tree node.
pub fn list_dir(path: &str) {
    send(&FsRequest::ListDir {
        request_id: 0,
        path: path.to_string(),
    });
}

/// Reads a file, to open it in a buffer.
pub fn read_file(path: &str) {
    send(&FsRequest::ReadFile {
        request_id: 0,
        path: path.to_string(),
    });
}

/// Writes a file's text to disk.
pub fn write_file(path: &str, text: String) {
    send(&FsRequest::WriteFile {
        request_id: 0,
        path: path.to_string(),
        text,
    });
}

/// Searches the workspace for a substring.
pub fn search(root: &str, query: &str) {
    send(&FsRequest::Search {
        request_id: 0,
        root: root.to_string(),
        query: query.to_string(),
    });
}

/// Creates an empty file and opens it.
pub fn create_path(path: &str) {
    send(&FsRequest::CreatePath {
        request_id: 0,
        path: path.to_string(),
    });
}

/// Renames or moves a path.
pub fn rename_path(from: &str, to: &str) {
    send(&FsRequest::RenamePath {
        request_id: 0,
        from: from.to_string(),
        to: to.to_string(),
    });
}

/// Deletes a file.
pub fn delete_path(path: &str) {
    send(&FsRequest::DeletePath {
        request_id: 0,
        path: path.to_string(),
    });
}

/// Replaces every regex match across the workspace.
pub fn replace_all(root: &str, query: &str, replacement: &str) {
    send(&FsRequest::ReplaceAll {
        request_id: 0,
        root: root.to_string(),
        query: query.to_string(),
        replacement: replacement.to_string(),
    });
}

/// Toggles a tree directory, loading its children on first expand.
pub fn toggle_dir(state: EditorState, path: &str) {
    let mut needs_load = false;
    state.explorer.tree.update(|nodes| {
        if let Some(node) = find_node(nodes, path) {
            node.expanded = !node.expanded;
            needs_load = node.expanded && node.children.is_empty();
        }
    });
    if needs_load {
        list_dir(path);
    }
}

fn dispatch(state: EditorState, response: FsResponse) {
    match response {
        FsResponse::Folder { root, entries, .. } => {
            state.explorer.root.set(root);
            state
                .explorer
                .tree
                .set(entries.into_iter().map(to_node).collect());
        }
        FsResponse::Dir { path, entries, .. } => {
            state.explorer.tree.update(|nodes| {
                if let Some(node) = find_node(nodes, &path) {
                    node.children = entries.into_iter().map(to_node).collect();
                    node.expanded = true;
                }
            });
        }
        FsResponse::File { path, text, .. } => {
            let already = state
                .files
                .with_untracked(|files| files.iter().any(|file| file.path == path));
            state.files.update(|files| {
                if let Some(file) = files.iter_mut().find(|file| file.path == path) {
                    file.text = text.clone();
                    file.dirty = false;
                } else {
                    files.push(FileBuffer {
                        path: path.clone(),
                        text: text.clone(),
                        dirty: false,
                    });
                }
            });
            if already {
                crate::lsp::did_change(state, &path);
            } else {
                state.open_in_focused(PluginKind::File, Some(path.clone()));
                crate::lsp::did_open(state, &path);
                crate::lsp::apply_pending_edits(state, &path);
            }
        }
        FsResponse::Wrote { path, .. } => {
            state.files.update(|files| {
                if let Some(file) = files.iter_mut().find(|file| file.path == path) {
                    file.dirty = false;
                }
            });
        }
        FsResponse::SearchResults { hits, .. } => {
            state.explorer.search_results.set(hits);
        }
        FsResponse::Replaced { count, .. } => {
            let paths: Vec<String> = state
                .files
                .with_untracked(|files| files.iter().map(|file| file.path.clone()).collect());
            for path in paths {
                read_file(&path);
            }
            state
                .editing
                .status
                .set(format!("Replaced across {count} files"));
        }
        FsResponse::Created {
            path, dir, entries, ..
        } => {
            refresh_dir(state, &dir, entries);
            read_file(&path);
        }
        FsResponse::Renamed {
            from,
            to,
            dir,
            entries,
            ..
        } => {
            refresh_dir(state, &dir, entries);
            state.files.update(|files| {
                if let Some(file) = files.iter_mut().find(|file| file.path == from) {
                    file.path = to.clone();
                }
            });
            state.panes.update(|panes| {
                for pane in panes.iter_mut() {
                    for tab in pane.tabs.iter_mut() {
                        if let TileContent::Buffer(buffer) = tab
                            && buffer.kind == PluginKind::File
                            && buffer.id.as_deref() == Some(from.as_str())
                        {
                            buffer.id = Some(to.clone());
                        }
                    }
                }
            });
        }
        FsResponse::Deleted {
            path, dir, entries, ..
        } => {
            refresh_dir(state, &dir, entries);
            state
                .files
                .update(|files| files.retain(|file| file.path != path));
            state.panes.update(|panes| {
                for pane in panes.iter_mut() {
                    pane.tabs.retain(|tab| {
                        !matches!(tab, TileContent::Buffer(buffer)
                            if buffer.kind == PluginKind::File
                                && buffer.id.as_deref() == Some(path.as_str()))
                    });
                    if pane.active >= pane.tabs.len() {
                        pane.active = pane.tabs.len().saturating_sub(1);
                    }
                }
            });
        }
        FsResponse::Error { message, .. } => {
            state.record_log([protocol::LogEntry {
                kind: LogKind::Error,
                label: "fs".to_string(),
                detail: message,
            }]);
        }
    }
}

/// Replaces a directory's children in the tree, or the whole tree when the
/// directory is the workspace root.
fn refresh_dir(state: EditorState, dir: &str, entries: Vec<DirEntry>) {
    if state.explorer.root.get_untracked().as_deref() == Some(dir) {
        state
            .explorer
            .tree
            .set(entries.into_iter().map(to_node).collect());
        return;
    }
    state.explorer.tree.update(|nodes| {
        if let Some(node) = find_node(nodes, dir) {
            node.children = entries.into_iter().map(to_node).collect();
            node.expanded = true;
        }
    });
}

fn to_node(entry: DirEntry) -> TreeNode {
    TreeNode {
        name: entry.name,
        path: entry.path,
        is_dir: entry.is_dir,
        expanded: false,
        children: Vec::new(),
    }
}

fn find_node<'a>(nodes: &'a mut [TreeNode], path: &str) -> Option<&'a mut TreeNode> {
    for node in nodes.iter_mut() {
        if node.path == path {
            return Some(node);
        }
        if node.is_dir
            && let Some(found) = find_node(&mut node.children, path)
        {
            return Some(found);
        }
    }
    None
}
