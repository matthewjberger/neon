//! The terminal bridge: a real pseudo-terminal. `portable-pty` spawns the shell
//! on a PTY (ConPTY on Windows, openpty elsewhere), `vt100` parses its byte
//! stream into a screen grid, and the grid streams to the page. The page sends
//! encoded keystrokes back as raw bytes and resizes drive both the PTY and the
//! emulator.

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use protocol::{TermGrid, TermSpan, TerminalClientMessage, TerminalServerMessage};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

const WS_ADDR: &str = "127.0.0.1:8794";

static STARTED: AtomicBool = AtomicBool::new(false);

/// Starts the terminal relay on a background thread. Idempotent.
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

    let mut session: Option<Session> = None;
    while let Some(Ok(message)) = source.next().await {
        let text = match message {
            Message::Text(text) => text,
            Message::Close(_) => break,
            _ => continue,
        };
        let Ok(request) = serde_json::from_str::<TerminalClientMessage>(&text) else {
            continue;
        };
        match request {
            TerminalClientMessage::Open { cols, rows, cwd } => {
                if session.is_none() {
                    session = open(cols, rows, cwd, out_tx.clone());
                }
            }
            TerminalClientMessage::Input { bytes } => {
                if let Some(session) = &session {
                    let _ = session.input.send(bytes);
                }
            }
            TerminalClientMessage::Resize { cols, rows } => {
                if let Some(session) = &session {
                    let _ = session.master.resize(PtySize {
                        rows,
                        cols,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                    if let Ok(mut parser) = session.parser.lock() {
                        parser.set_size(rows, cols);
                    }
                }
            }
        }
    }
    writer.abort();
}

/// A live PTY: the master for resizing, the emulator, and the input channel.
struct Session {
    master: Box<dyn portable_pty::MasterPty + Send>,
    parser: Arc<Mutex<vt100::Parser>>,
    input: std::sync::mpsc::Sender<Vec<u8>>,
}

fn open(
    cols: u16,
    rows: u16,
    cwd: String,
    out_tx: mpsc::UnboundedSender<String>,
) -> Option<Session> {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .ok()?;
    let mut command = CommandBuilder::new(shell());
    if !cwd.is_empty() {
        command.cwd(cwd);
    }
    let mut child = pair.slave.spawn_command(command).ok()?;
    drop(pair.slave);
    let reader = pair.master.try_clone_reader().ok()?;
    let mut writer = pair.master.take_writer().ok()?;

    let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 0)));

    let read_parser = parser.clone();
    let read_tx = out_tx.clone();
    std::thread::spawn(move || read_loop(reader, read_parser, read_tx));

    let (input, input_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        while let Ok(bytes) = input_rx.recv() {
            if writer.write_all(&bytes).is_err() {
                break;
            }
            let _ = writer.flush();
        }
    });

    std::thread::spawn(move || {
        let _ = child.wait();
        if let Ok(text) = serde_json::to_string(&TerminalServerMessage::Exited) {
            let _ = out_tx.send(text);
        }
    });

    Some(Session {
        master: pair.master,
        parser,
        input,
    })
}

fn read_loop(
    mut reader: Box<dyn Read + Send>,
    parser: Arc<Mutex<vt100::Parser>>,
    out_tx: mpsc::UnboundedSender<String>,
) {
    let mut buffer = [0_u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(count) => {
                let grid = {
                    let Ok(mut parser) = parser.lock() else {
                        break;
                    };
                    parser.process(&buffer[..count]);
                    build_grid(parser.screen())
                };
                if let Ok(text) = serde_json::to_string(&TerminalServerMessage::Grid(grid)) {
                    if out_tx.send(text).is_err() {
                        break;
                    }
                }
            }
        }
    }
}

fn build_grid(screen: &vt100::Screen) -> TermGrid {
    let (rows, cols) = screen.size();
    let (cursor_row, cursor_col) = screen.cursor_position();
    let mut lines = Vec::with_capacity(rows as usize);
    for row in 0..rows {
        let mut spans: Vec<TermSpan> = Vec::new();
        for col in 0..cols {
            let (text, span) = match screen.cell(row, col) {
                Some(cell) => {
                    let contents = cell.contents();
                    let text = if contents.is_empty() {
                        " ".to_string()
                    } else {
                        contents
                    };
                    (
                        text,
                        TermSpan {
                            text: String::new(),
                            fg: color_css(cell.fgcolor()),
                            bg: color_css(cell.bgcolor()),
                            bold: cell.bold(),
                            italic: cell.italic(),
                            underline: cell.underline(),
                            inverse: cell.inverse(),
                        },
                    )
                }
                None => (" ".to_string(), TermSpan::default()),
            };
            match spans.last_mut() {
                Some(last) if same_style(last, &span) => last.text.push_str(&text),
                _ => spans.push(TermSpan { text, ..span }),
            }
        }
        lines.push(spans);
    }
    TermGrid {
        cols,
        rows,
        cursor_row,
        cursor_col,
        cursor_visible: !screen.hide_cursor(),
        lines,
    }
}

fn same_style(left: &TermSpan, right: &TermSpan) -> bool {
    left.fg == right.fg
        && left.bg == right.bg
        && left.bold == right.bold
        && left.italic == right.italic
        && left.underline == right.underline
        && left.inverse == right.inverse
}

fn color_css(color: vt100::Color) -> String {
    match color {
        vt100::Color::Default => String::new(),
        vt100::Color::Idx(index) => palette(index),
        vt100::Color::Rgb(red, green, blue) => format!("#{red:02x}{green:02x}{blue:02x}"),
    }
}

fn palette(index: u8) -> String {
    const BASE: [&str; 16] = [
        "#000000", "#cd0000", "#00cd00", "#cdcd00", "#1e90ff", "#cd00cd", "#00cdcd", "#e5e5e5",
        "#7f7f7f", "#ff0000", "#00ff00", "#ffff00", "#5c5cff", "#ff00ff", "#00ffff", "#ffffff",
    ];
    if (index as usize) < 16 {
        return BASE[index as usize].to_string();
    }
    if index >= 232 {
        let value = 8 + (index - 232) as u16 * 10;
        return format!("#{value:02x}{value:02x}{value:02x}");
    }
    let cube = index - 16;
    let component = |value: u8| -> u8 { if value == 0 { 0 } else { 55 + value * 40 } };
    let red = component(cube / 36);
    let green = component((cube % 36) / 6);
    let blue = component(cube % 6);
    format!("#{red:02x}{green:02x}{blue:02x}")
}

fn shell() -> String {
    if cfg!(windows) {
        "cmd.exe".to_string()
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string())
    }
}
