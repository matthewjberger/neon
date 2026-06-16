//! The chat bridge: a websocket relay that shuttles page chat messages into a
//! persistent Claude Code subprocess over stream-json stdin, and forwards every
//! stdout and stderr line back to the page. The subprocess is pointed at the
//! in-process MCP endpoint, so it can drive the editor it is chatting inside of.

use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::tungstenite::Message;

const WS_ADDR: &str = "127.0.0.1:8791";
const MCP_URL: &str = "http://127.0.0.1:8790/mcp";
const SYSTEM_PROMPT: &str = "You are embedded in the Neon editor's chat panel. You drive the editor only through the neon MCP tools. You have no filesystem, shell, or web access, so do not try to read the source: call get_api_reference to learn the scripting API (every command and standard-library helper). A plugin is a rhai script with on_start and/or on_tick that pushes Commands to `commands` and reads this frame's Events from `events`. Author and edit plugins with edit_plugin and set_buffer; both return diagnostics, so check that ok is true and fix any errors or unknown-command warnings before moving on. After editing, call get_console to see runtime errors and the commands a plugin ran, and query_scene to confirm entities exist. Use get_editor_state for the open plugins and selection, and screenshot to see the viewport.";

struct Shared {
    page_tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
    claude_stdin: Mutex<Option<ChildStdin>>,
    generation: AtomicU64,
}

static STARTED: AtomicBool = AtomicBool::new(false);

/// Starts the chat relay on a background thread hosting its own tokio runtime.
/// Idempotent, so the page can re-send its open signal whenever the chat opens.
pub fn start() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                log(&format!("failed to start the chat runtime: {error}"));
                return;
            }
        };
        let shared = Arc::new(Shared {
            page_tx: Mutex::new(None),
            claude_stdin: Mutex::new(None),
            generation: AtomicU64::new(0),
        });
        runtime.block_on(run_ws_server(shared));
    });
}

async fn run_ws_server(shared: Arc<Shared>) {
    let listener = match tokio::net::TcpListener::bind(WS_ADDR).await {
        Ok(listener) => listener,
        Err(error) => {
            log(&format!("failed to bind {WS_ADDR}: {error}"));
            return;
        }
    };
    log(&format!("chat relay listening on ws://{WS_ADDR}"));
    loop {
        let Ok((stream, _addr)) = listener.accept().await else {
            continue;
        };
        let connection_shared = shared.clone();
        tokio::spawn(async move {
            handle_page(connection_shared, stream).await;
        });
    }
}

async fn handle_page(shared: Arc<Shared>, stream: tokio::net::TcpStream) {
    let websocket = match tokio_tungstenite::accept_async(stream).await {
        Ok(websocket) => websocket,
        Err(error) => {
            log(&format!("chat handshake failed: {error}"));
            return;
        }
    };
    log("chat page connected");
    let (mut sink, mut source) = websocket.split();

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();
    *shared.page_tx.lock().await = Some(out_tx);

    let writer = tokio::spawn(async move {
        while let Some(text) = out_rx.recv().await {
            if sink.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    });

    ensure_claude(&shared).await;

    while let Some(message) = source.next().await {
        let Ok(message) = message else {
            break;
        };
        let text = match message {
            Message::Text(text) => text,
            Message::Close(_) => break,
            _ => continue,
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        match value.get("type").and_then(Value::as_str) {
            Some("user") => {
                if let Some(prompt) = value.get("text").and_then(Value::as_str) {
                    forward_prompt(&shared, prompt).await;
                }
            }
            Some("restart") => {
                shared.claude_stdin.lock().await.take();
                ensure_claude(&shared).await;
            }
            _ => {}
        }
    }

    *shared.page_tx.lock().await = None;
    shared.claude_stdin.lock().await.take();
    writer.abort();
    log("chat page disconnected");
}

async fn ensure_claude(shared: &Arc<Shared>) {
    let mut stdin_slot = shared.claude_stdin.lock().await;
    if stdin_slot.is_some() {
        return;
    }
    let mcp_config = json!({
        "mcpServers": { "neon": { "type": "http", "url": MCP_URL } }
    })
    .to_string();
    let mut command = Command::new("claude");
    command
        .arg("--print")
        .arg("--input-format")
        .arg("stream-json")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--permission-mode")
        .arg("dontAsk")
        .arg("--allowed-tools")
        .arg("mcp__neon__*")
        .arg("--disallowed-tools")
        .arg("Bash Edit Write Read WebFetch WebSearch Task Glob Grep NotebookEdit")
        .arg("--mcp-config")
        .arg(mcp_config)
        .arg("--append-system-prompt")
        .arg(SYSTEM_PROMPT)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            drop(stdin_slot);
            send_to_page(
                shared,
                json!({ "type": "stderr", "text": format!("failed to launch claude: {error}") })
                    .to_string(),
            )
            .await;
            return;
        }
    };
    let generation = shared.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    *stdin_slot = child.stdin.take();
    drop(stdin_slot);
    log("claude session started");

    if let Some(stdout) = stdout {
        let stdout_shared = shared.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                send_to_page(&stdout_shared, line).await;
            }
        });
    }
    if let Some(stderr) = stderr {
        let stderr_shared = shared.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                send_to_page(
                    &stderr_shared,
                    json!({ "type": "stderr", "text": line }).to_string(),
                )
                .await;
            }
        });
    }
    let exit_shared = shared.clone();
    tokio::spawn(async move {
        let status = child.wait().await;
        if exit_shared.generation.load(Ordering::SeqCst) != generation {
            return;
        }
        exit_shared.claude_stdin.lock().await.take();
        let code = status.ok().and_then(|status| status.code());
        send_to_page(
            &exit_shared,
            json!({ "type": "process_exit", "code": code }).to_string(),
        )
        .await;
    });
}

async fn forward_prompt(shared: &Arc<Shared>, prompt: &str) {
    ensure_claude(shared).await;
    let line = json!({
        "type": "user",
        "message": { "role": "user", "content": [{ "type": "text", "text": prompt }] }
    })
    .to_string();
    let mut stdin_slot = shared.claude_stdin.lock().await;
    let Some(stdin) = stdin_slot.as_mut() else {
        return;
    };
    let failed = stdin.write_all(line.as_bytes()).await.is_err()
        || stdin.write_all(b"\n").await.is_err()
        || stdin.flush().await.is_err();
    if failed {
        stdin_slot.take();
        drop(stdin_slot);
        send_to_page(
            shared,
            json!({ "type": "stderr", "text": "lost the claude session, send again to restart it" })
                .to_string(),
        )
        .await;
    }
}

async fn send_to_page(shared: &Arc<Shared>, text: String) {
    let guard = shared.page_tx.lock().await;
    if let Some(sender) = guard.as_ref() {
        let _ = sender.send(text);
    }
}

fn log(message: &str) {
    eprintln!("[chat] {message}");
}
