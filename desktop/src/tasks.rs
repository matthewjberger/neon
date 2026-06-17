//! The task runner bridge: spawns a process (cargo and friends) in the
//! workspace and streams its output to the page line by line, with a cancel.
//! This is the basis of the Rust dev loop: run, build, test, and check.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::{SinkExt, StreamExt};
use protocol::{TaskRequest, TaskResponse};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;

const WS_ADDR: &str = "127.0.0.1:8794";

static STARTED: AtomicBool = AtomicBool::new(false);

/// Starts the task runner relay on a background thread. Idempotent.
pub fn start() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        if let Ok(runtime) = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            runtime.block_on(run_server());
        }
    });
}

async fn run_server() {
    let Ok(listener) = tokio::net::TcpListener::bind(WS_ADDR).await else {
        return;
    };
    loop {
        let Ok((stream, _addr)) = listener.accept().await else {
            continue;
        };
        tokio::spawn(handle_page(stream));
    }
}

async fn handle_page(stream: tokio::net::TcpStream) {
    let Ok(websocket) = tokio_tungstenite::accept_async(stream).await else {
        return;
    };
    let (mut sink, mut source) = websocket.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();
    let writer = tokio::spawn(async move {
        while let Some(text) = out_rx.recv().await {
            if sink.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    });
    let cancels: Arc<Mutex<HashMap<u64, oneshot::Sender<()>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    while let Some(Ok(message)) = source.next().await {
        let text = match message {
            Message::Text(text) => text,
            Message::Close(_) => break,
            _ => continue,
        };
        let Ok(request) = serde_json::from_str::<TaskRequest>(&text) else {
            continue;
        };
        match request {
            TaskRequest::Run {
                id,
                program,
                args,
                cwd,
            } => {
                let (cancel_tx, cancel_rx) = oneshot::channel();
                cancels.lock().await.insert(id, cancel_tx);
                let out_tx = out_tx.clone();
                let cancels = cancels.clone();
                tokio::spawn(async move {
                    run_task(id, program, args, cwd, out_tx, cancel_rx).await;
                    cancels.lock().await.remove(&id);
                });
            }
            TaskRequest::Shell { id, command, cwd } => {
                let (program, args) = if cfg!(windows) {
                    ("cmd".to_string(), vec!["/C".to_string(), command])
                } else {
                    ("sh".to_string(), vec!["-c".to_string(), command])
                };
                let (cancel_tx, cancel_rx) = oneshot::channel();
                cancels.lock().await.insert(id, cancel_tx);
                let out_tx = out_tx.clone();
                let cancels = cancels.clone();
                tokio::spawn(async move {
                    run_task(id, program, args, cwd, out_tx, cancel_rx).await;
                    cancels.lock().await.remove(&id);
                });
            }
            TaskRequest::Cancel { id } => {
                if let Some(sender) = cancels.lock().await.remove(&id) {
                    let _ = sender.send(());
                }
            }
        }
    }
    writer.abort();
}

async fn run_task(
    id: u64,
    program: String,
    args: Vec<String>,
    cwd: String,
    out_tx: mpsc::UnboundedSender<String>,
    cancel_rx: oneshot::Receiver<()>,
) {
    let mut command = Command::new(&program);
    command
        .args(&args)
        .current_dir(&cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            send(
                &out_tx,
                &TaskResponse::Error {
                    id,
                    message: format!("failed to start {program}: {error}"),
                },
            );
            return;
        }
    };
    send(
        &out_tx,
        &TaskResponse::Started {
            id,
            label: format!("{program} {}", args.join(" ")),
        },
    );
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_tx = out_tx.clone();
    let stdout_task = tokio::spawn(async move {
        if let Some(stream) = stdout {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                send(
                    &stdout_tx,
                    &TaskResponse::Line {
                        id,
                        text: strip_ansi(&line),
                    },
                );
            }
        }
    });
    let stderr_tx = out_tx.clone();
    let stderr_task = tokio::spawn(async move {
        if let Some(stream) = stderr {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                send(
                    &stderr_tx,
                    &TaskResponse::Line {
                        id,
                        text: strip_ansi(&line),
                    },
                );
            }
        }
    });
    tokio::select! {
        status = child.wait() => {
            let _ = stdout_task.await;
            let _ = stderr_task.await;
            let code = status.ok().and_then(|status| status.code());
            send(&out_tx, &TaskResponse::Exited { id, code });
        }
        _ = cancel_rx => {
            let _ = child.start_kill();
            let _ = child.wait().await;
            stdout_task.abort();
            stderr_task.abort();
            send(&out_tx, &TaskResponse::Exited { id, code: None });
        }
    }
}

fn send(out_tx: &mpsc::UnboundedSender<String>, message: &TaskResponse) {
    if let Ok(text) = serde_json::to_string(message) {
        let _ = out_tx.send(text);
    }
}

/// Strips ANSI escape sequences so cargo's colored output reads cleanly.
fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
            }
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
        } else {
            out.push(character);
        }
    }
    out
}
