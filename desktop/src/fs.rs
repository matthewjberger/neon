//! The filesystem bridge: a websocket relay that gives the page disk access it
//! cannot have on its own. Each `FsRequest` runs natively here (a folder picker,
//! a directory list, a file read or write) and the `FsResponse` goes back on the
//! same socket, matched by the page through the request id.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::{SinkExt, StreamExt};
use protocol::{DirEntry, FsRequest, FsResponse};
use tokio_tungstenite::tungstenite::Message;

const WS_ADDR: &str = "127.0.0.1:8792";

static STARTED: AtomicBool = AtomicBool::new(false);

/// Starts the filesystem relay on a background thread with its own runtime.
/// Idempotent.
pub fn start() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime.block_on(run_server()),
            Err(error) => log(&format!("failed to start the fs runtime: {error}")),
        }
    });
}

async fn run_server() {
    let listener = match tokio::net::TcpListener::bind(WS_ADDR).await {
        Ok(listener) => listener,
        Err(error) => {
            log(&format!("failed to bind {WS_ADDR}: {error}"));
            return;
        }
    };
    log(&format!("filesystem relay listening on ws://{WS_ADDR}"));
    loop {
        let Ok((stream, _addr)) = listener.accept().await else {
            continue;
        };
        tokio::spawn(async move {
            handle_page(stream).await;
        });
    }
}

async fn handle_page(stream: tokio::net::TcpStream) {
    let websocket = match tokio_tungstenite::accept_async(stream).await {
        Ok(websocket) => websocket,
        Err(error) => {
            log(&format!("fs handshake failed: {error}"));
            return;
        }
    };
    let (mut sink, mut source) = websocket.split();
    while let Some(message) = source.next().await {
        let Ok(message) = message else {
            break;
        };
        let text = match message {
            Message::Text(text) => text,
            Message::Close(_) => break,
            _ => continue,
        };
        let Ok(request) = serde_json::from_str::<FsRequest>(&text) else {
            continue;
        };
        let response = handle(request).await;
        let Ok(json) = serde_json::to_string(&response) else {
            continue;
        };
        if sink.send(Message::Text(json)).await.is_err() {
            break;
        }
    }
}

async fn handle(request: FsRequest) -> FsResponse {
    match request {
        FsRequest::OpenFolder { request_id } => {
            let picked = tokio::task::spawn_blocking(|| rfd::FileDialog::new().pick_folder())
                .await
                .ok()
                .flatten();
            match picked {
                Some(path) => {
                    let root = path.to_string_lossy().to_string();
                    let entries = list_dir(&path).await.unwrap_or_default();
                    FsResponse::Folder {
                        request_id,
                        root: Some(root),
                        entries,
                    }
                }
                None => FsResponse::Folder {
                    request_id,
                    root: None,
                    entries: Vec::new(),
                },
            }
        }
        FsRequest::ListDir { request_id, path } => match list_dir(Path::new(&path)).await {
            Ok(entries) => FsResponse::Dir {
                request_id,
                path,
                entries,
            },
            Err(message) => FsResponse::Error {
                request_id,
                message,
            },
        },
        FsRequest::ReadFile { request_id, path } => match tokio::fs::read_to_string(&path).await {
            Ok(text) => FsResponse::File {
                request_id,
                path,
                text,
            },
            Err(error) => FsResponse::Error {
                request_id,
                message: error.to_string(),
            },
        },
        FsRequest::WriteFile {
            request_id,
            path,
            text,
        } => match tokio::fs::write(&path, text).await {
            Ok(()) => FsResponse::Wrote { request_id, path },
            Err(error) => FsResponse::Error {
                request_id,
                message: error.to_string(),
            },
        },
        FsRequest::Search {
            request_id,
            root,
            query,
        } => {
            let hits = tokio::task::spawn_blocking(move || search(&root, &query))
                .await
                .unwrap_or_default();
            FsResponse::SearchResults { request_id, hits }
        }
    }
}

const SEARCH_LIMIT: usize = 500;

fn search(root: &str, query: &str) -> Vec<protocol::SearchHit> {
    if query.is_empty() {
        return Vec::new();
    }
    let needle = query.to_lowercase();
    let mut hits = Vec::new();
    for entry in ignore::WalkBuilder::new(root)
        .hidden(true)
        .build()
        .flatten()
    {
        if hits.len() >= SEARCH_LIMIT {
            break;
        }
        if !entry
            .file_type()
            .is_some_and(|file_type| file_type.is_file())
        {
            continue;
        }
        let path = entry.path();
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        for (number, line) in text.lines().enumerate() {
            if line.to_lowercase().contains(&needle) {
                hits.push(protocol::SearchHit {
                    path: path.to_string_lossy().to_string(),
                    line: number as u32 + 1,
                    text: line.trim().chars().take(200).collect(),
                });
                if hits.len() >= SEARCH_LIMIT {
                    break;
                }
            }
        }
    }
    hits
}

async fn list_dir(path: &Path) -> Result<Vec<DirEntry>, String> {
    let mut reader = tokio::fs::read_dir(path)
        .await
        .map_err(|error| error.to_string())?;
    let mut entries = Vec::new();
    while let Some(entry) = reader
        .next_entry()
        .await
        .map_err(|error| error.to_string())?
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "target" {
            continue;
        }
        let is_dir = entry
            .file_type()
            .await
            .map(|file_type| file_type.is_dir())
            .unwrap_or(false);
        entries.push(DirEntry {
            name,
            path: entry.path().to_string_lossy().to_string(),
            is_dir,
        });
    }
    entries.sort_by(|left, right| {
        right
            .is_dir
            .cmp(&left.is_dir)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(entries)
}

fn log(message: &str) {
    eprintln!("[fs] {message}");
}
