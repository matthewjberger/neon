//! The language-server bridge: a websocket relay that spawns the language server
//! for the workspace's language family and shuttles LSP JSON-RPC between it and
//! the page, which is the LSP client. The server speaks `Content-Length` framed
//! messages over stdio, so this frames outgoing messages and reframes incoming
//! ones into whole JSON payloads. Rust is discovered through rustup; the other
//! servers (typescript-language-server, pyright, gopls, clangd) are taken from
//! PATH.

use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::StreamExt;
use protocol::{LspClientMessage, LspServerMessage};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::tungstenite::Message;

const WS_ADDR: &str = "127.0.0.1:8793";

struct Shared {
    page_tx: Mutex<Option<mpsc::Sender<String>>>,
    server_stdin: Mutex<Option<ChildStdin>>,
}

static STARTED: AtomicBool = AtomicBool::new(false);

/// Starts the language-server relay on a background thread with its own runtime.
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
            Ok(runtime) => {
                let shared = Arc::new(Shared {
                    page_tx: Mutex::new(None),
                    server_stdin: Mutex::new(None),
                });
                runtime.block_on(run_server(shared));
            }
            Err(error) => log(&format!("failed to start the lsp runtime: {error}")),
        }
    });
}

async fn run_server(shared: Arc<Shared>) {
    let listener = match tokio::net::TcpListener::bind(WS_ADDR).await {
        Ok(listener) => listener,
        Err(error) => {
            log(&format!("failed to bind {WS_ADDR}: {error}"));
            return;
        }
    };
    log(&format!("lsp relay listening on ws://{WS_ADDR}"));
    loop {
        let Ok((stream, _addr)) = listener.accept().await else {
            continue;
        };
        let connection = shared.clone();
        tokio::spawn(async move {
            handle_page(connection, stream).await;
        });
    }
}

async fn handle_page(shared: Arc<Shared>, stream: tokio::net::TcpStream) {
    let websocket = match tokio_tungstenite::accept_async(stream).await {
        Ok(websocket) => websocket,
        Err(error) => {
            log(&format!("lsp handshake failed: {error}"));
            return;
        }
    };
    let (sink, mut source) = websocket.split();

    let (out_tx, out_rx) = mpsc::channel::<String>(crate::relay::PAGE_QUEUE);
    *shared.page_tx.lock().await = Some(out_tx);
    let writer = crate::relay::spawn_writer(sink, out_rx);

    while let Some(message) = source.next().await {
        let Ok(message) = message else {
            break;
        };
        let text = match message {
            Message::Text(text) => text,
            Message::Close(_) => break,
            _ => continue,
        };
        let Ok(message) = serde_json::from_str::<LspClientMessage>(&text) else {
            continue;
        };
        match message {
            LspClientMessage::Start { language, .. } => ensure_server(&shared, &language).await,
            LspClientMessage::Rpc { json } => forward_rpc(&shared, &json).await,
            LspClientMessage::Stop => {
                shared.server_stdin.lock().await.take();
            }
        }
    }

    *shared.page_tx.lock().await = None;
    shared.server_stdin.lock().await.take();
    writer.abort();
}

async fn ensure_server(shared: &Arc<Shared>, language: &str) {
    {
        if shared.server_stdin.lock().await.is_some() {
            return;
        }
    }
    let (program, args) = match discover(language).await {
        Ok(resolved) => resolved,
        Err(error) => {
            send_to_page(shared, &LspServerMessage::Error { message: error }).await;
            return;
        }
    };
    let mut command = Command::new(&program);
    command
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            send_to_page(
                shared,
                &LspServerMessage::Error {
                    message: format!("failed to launch {program}: {error}"),
                },
            )
            .await;
            return;
        }
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    *shared.server_stdin.lock().await = child.stdin.take();
    send_to_page(shared, &LspServerMessage::Started).await;
    log(&format!("{program} started"));

    if let Some(stdout) = stdout {
        let stdout_shared = shared.clone();
        tokio::spawn(async move {
            read_frames(stdout_shared, stdout).await;
        });
    }
    if let Some(stderr) = stderr {
        let stderr_shared = shared.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                send_to_page(&stderr_shared, &LspServerMessage::Log { line }).await;
            }
        });
    }
    let exit_shared = shared.clone();
    tokio::spawn(async move {
        let status = child.wait().await;
        exit_shared.server_stdin.lock().await.take();
        let code = status.ok().and_then(|status| status.code());
        send_to_page(&exit_shared, &LspServerMessage::Exited { code }).await;
    });
}

/// Reads `Content-Length` framed JSON-RPC from the server's stdout and forwards
/// each whole message to the page.
async fn read_frames(shared: Arc<Shared>, stdout: tokio::process::ChildStdout) {
    let mut reader = BufReader::new(stdout);
    loop {
        let mut content_length = 0_usize;
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => return,
                Ok(_) => {}
                Err(_) => return,
            }
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                break;
            }
            if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
                content_length = rest.trim().parse().unwrap_or(0);
            }
        }
        if content_length == 0 {
            continue;
        }
        let mut buffer = vec![0_u8; content_length];
        if reader.read_exact(&mut buffer).await.is_err() {
            return;
        }
        let json = String::from_utf8_lossy(&buffer).to_string();
        send_to_page(&shared, &LspServerMessage::Rpc { json }).await;
    }
}

async fn forward_rpc(shared: &Arc<Shared>, json: &str) {
    let mut stdin_slot = shared.server_stdin.lock().await;
    let Some(stdin) = stdin_slot.as_mut() else {
        return;
    };
    let header = format!("Content-Length: {}\r\n\r\n", json.len());
    let failed = stdin.write_all(header.as_bytes()).await.is_err()
        || stdin.write_all(json.as_bytes()).await.is_err()
        || stdin.flush().await.is_err();
    if failed {
        stdin_slot.take();
    }
}

/// Resolves the server program and arguments for a language family. Rust goes
/// through rustup (installing the component if needed); the rest are expected on
/// PATH, the standard install for each toolchain.
async fn discover(language: &str) -> Result<(String, Vec<String>), String> {
    match language {
        "rust" => Ok((discover_rust_analyzer().await, Vec::new())),
        "typescript" => Ok((
            "typescript-language-server".to_string(),
            vec!["--stdio".to_string()],
        )),
        "python" => Ok((
            "pyright-langserver".to_string(),
            vec!["--stdio".to_string()],
        )),
        "go" => Ok(("gopls".to_string(), Vec::new())),
        "cpp" => Ok(("clangd".to_string(), Vec::new())),
        other => Err(format!("no language server configured for {other}")),
    }
}

/// Finds rust-analyzer through rustup, installing the component if needed, then
/// falls back to the binary on PATH.
async fn discover_rust_analyzer() -> String {
    if let Some(path) = rustup_which().await {
        return path;
    }
    let _ = Command::new("rustup")
        .arg("component")
        .arg("add")
        .arg("rust-analyzer")
        .status()
        .await;
    if let Some(path) = rustup_which().await {
        return path;
    }
    "rust-analyzer".to_string()
}

async fn rustup_which() -> Option<String> {
    let output = Command::new("rustup")
        .arg("which")
        .arg("rust-analyzer")
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() { None } else { Some(path) }
}

async fn send_to_page(shared: &Arc<Shared>, message: &LspServerMessage) {
    let Ok(text) = serde_json::to_string(message) else {
        return;
    };
    let sender = shared.page_tx.lock().await.clone();
    if let Some(sender) = sender {
        let _ = sender.send(text).await;
    }
}

fn log(message: &str) {
    eprintln!("[lsp] {message}");
}
