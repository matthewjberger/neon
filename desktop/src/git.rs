//! The git bridge: a websocket relay that diffs a file against HEAD and hands
//! the page the changed line numbers for its gutter. It shells out to `git`
//! (`git diff --unified=0`) in the file's directory and parses the hunk headers,
//! so it needs only git on PATH, no library.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::{SinkExt, StreamExt};
use protocol::{GitChange, GitClientMessage, GitFile, GitServerMessage};
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
    match request {
        GitClientMessage::DiffFile { request_id, path } => {
            let changes = diff(&path).await.unwrap_or_default();
            GitServerMessage::Diff {
                request_id,
                path,
                changes,
            }
        }
        GitClientMessage::Status { request_id, root } => {
            let (branch, files) = status(&root).await.unwrap_or_default();
            GitServerMessage::Status {
                request_id,
                branch,
                files,
            }
        }
        GitClientMessage::Stage {
            request_id,
            root,
            path,
        } => {
            run_git(&root, &["add", "--", &path]).await;
            GitServerMessage::Done { request_id }
        }
        GitClientMessage::Unstage {
            request_id,
            root,
            path,
        } => {
            run_git(&root, &["reset", "-q", "HEAD", "--", &path]).await;
            GitServerMessage::Done { request_id }
        }
        GitClientMessage::Commit {
            request_id,
            root,
            message,
        } => {
            run_git(&root, &["commit", "-m", &message]).await;
            GitServerMessage::Done { request_id }
        }
    }
}

/// Runs a git subcommand in a repo, ignoring its output.
async fn run_git(root: &str, args: &[&str]) {
    let _ = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .await;
}

/// The repo's branch and changed files from `git status --porcelain -b`.
async fn status(root: &str) -> Option<(String, Vec<GitFile>)> {
    let output = Command::new("git")
        .current_dir(root)
        .arg("status")
        .arg("--porcelain=v1")
        .arg("-b")
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return Some((String::new(), Vec::new()));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Some(parse_status(&text))
}

/// Parses porcelain status into the branch name and per-path change entries.
fn parse_status(text: &str) -> (String, Vec<GitFile>) {
    let mut branch = String::new();
    let mut files = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            branch = rest
                .split(['.', ' '])
                .next()
                .unwrap_or_default()
                .to_string();
            continue;
        }
        if line.len() < 3 {
            continue;
        }
        let index = line.chars().next().unwrap_or(' ');
        let worktree = line.chars().nth(1).unwrap_or(' ');
        let path = line[3..].to_string();
        if index != ' ' && index != '?' {
            files.push(GitFile {
                path: path.clone(),
                staged: true,
                status: index.to_string(),
            });
        }
        if worktree != ' ' {
            files.push(GitFile {
                path,
                staged: false,
                status: worktree.to_string(),
            });
        }
    }
    (branch, files)
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
