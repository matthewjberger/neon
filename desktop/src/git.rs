//! The git bridge: a websocket relay that diffs a file against HEAD and hands
//! the page the changed line numbers for its gutter. It shells out to `git`
//! (`git diff --unified=0`) in the file's directory and parses the hunk headers,
//! so it needs only git on PATH, no library.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::{SinkExt, StreamExt};
use protocol::{GitChange, GitClientMessage, GitServerMessage};
use tokio::process::Command;
use tokio_tungstenite::tungstenite::Message;

const WS_ADDR: &str = "127.0.0.1:8795";

static STARTED: AtomicBool = AtomicBool::new(false);

/// Starts the git relay on a background thread with its own runtime. Idempotent.
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
            Err(error) => log(&format!("failed to start the git runtime: {error}")),
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
    log(&format!("git relay listening on ws://{WS_ADDR}"));
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
            log(&format!("git handshake failed: {error}"));
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
        let Ok(request) = serde_json::from_str::<GitClientMessage>(&text) else {
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

async fn handle(request: GitClientMessage) -> GitServerMessage {
    let GitClientMessage::DiffFile { request_id, path } = request;
    let changes = diff(&path).await.unwrap_or_default();
    GitServerMessage::Diff {
        request_id,
        path,
        changes,
    }
}

/// Runs `git diff` for one file and turns its hunk headers into per-line change
/// marks. Returns none when the file is not in a repo or git is absent.
async fn diff(path: &str) -> Option<Vec<(u32, GitChange)>> {
    let dir = Path::new(path).parent()?;
    let output = Command::new("git")
        .current_dir(dir)
        .arg("diff")
        .arg("--no-color")
        .arg("--unified=0")
        .arg("--")
        .arg(path)
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return Some(Vec::new());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Some(parse_hunks(&text))
}

/// Parses `@@ -old +new @@` headers into the changed lines of the new file.
fn parse_hunks(diff: &str) -> Vec<(u32, GitChange)> {
    let mut changes = Vec::new();
    for line in diff.lines() {
        let Some(rest) = line.strip_prefix("@@ ") else {
            continue;
        };
        let Some(header) = rest.split(" @@").next() else {
            continue;
        };
        let mut parts = header.split_whitespace();
        let old = parts.next().and_then(|part| part.strip_prefix('-'));
        let new = parts.next().and_then(|part| part.strip_prefix('+'));
        let (Some(old), Some(new)) = (old, new) else {
            continue;
        };
        let (_, old_count) = range(old);
        let (new_start, new_count) = range(new);
        if old_count == 0 {
            for offset in 0..new_count {
                changes.push((new_start + offset, GitChange::Added));
            }
        } else if new_count == 0 {
            changes.push((new_start.max(1), GitChange::Removed));
        } else {
            for offset in 0..new_count {
                changes.push((new_start + offset, GitChange::Modified));
            }
        }
    }
    changes
}

/// A diff range `start,count`, with the count defaulting to 1 when omitted.
fn range(part: &str) -> (u32, u32) {
    let mut fields = part.split(',');
    let start = fields.next().and_then(|n| n.parse().ok()).unwrap_or(0);
    let count = fields.next().and_then(|n| n.parse().ok()).unwrap_or(1);
    (start, count)
}

fn log(message: &str) {
    eprintln!("[git] {message}");
}
