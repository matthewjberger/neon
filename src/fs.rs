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

use crate::state::{EditorState, FileBuffer, PluginKind, TreeNode};

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

/// Toggles a tree directory, loading its children on first expand.
pub fn toggle_dir(state: EditorState, path: &str) {
    let mut needs_load = false;
    state.tree.update(|nodes| {
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
            state.workspace_root.set(root);
            state.tree.set(entries.into_iter().map(to_node).collect());
        }
        FsResponse::Dir { path, entries, .. } => {
            state.tree.update(|nodes| {
                if let Some(node) = find_node(nodes, &path) {
                    node.children = entries.into_iter().map(to_node).collect();
                    node.expanded = true;
                }
            });
        }
        FsResponse::File { path, text, .. } => {
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
            state.open_in_focused(PluginKind::File, Some(path.clone()));
            crate::lsp::did_open(state, &path);
            crate::lsp::apply_pending_edits(state, &path);
        }
        FsResponse::Wrote { path, .. } => {
            state.files.update(|files| {
                if let Some(file) = files.iter_mut().find(|file| file.path == path) {
                    file.dirty = false;
                }
            });
        }
        FsResponse::SearchResults { hits, .. } => {
            state.search_results.set(hits);
        }
        FsResponse::Error { message, .. } => {
            state.log.update(|log| {
                log.push(protocol::LogEntry {
                    kind: LogKind::Error,
                    label: "fs".to_string(),
                    detail: message,
                });
            });
        }
    }
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
