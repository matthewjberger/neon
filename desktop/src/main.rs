//! Standalone shell: hosts the same web bundle the browser runs, served from
//! a local port into a native webview window. Debug builds read `../dist`
//! from disk so a fresh `trunk build` shows up on relaunch; release builds
//! embed the bundle into the executable. With the `networking` feature the
//! first process hosts a broker and opens more windows as child processes.

#[cfg(feature = "agent")]
mod agent;
#[cfg(feature = "agent")]
mod chat;
#[cfg(feature = "agent")]
mod fs;
#[cfg(feature = "agent")]
mod git;
#[cfg(feature = "agent")]
mod highlight;
#[cfg(feature = "agent")]
mod lsp;
#[cfg(feature = "networking")]
mod network;
#[cfg(feature = "agent")]
mod pty;
#[cfg(feature = "agent")]
mod relay;

use rust_embed::RustEmbed;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use wry::{WebView, WebViewBuilder};

#[derive(RustEmbed)]
#[folder = "../dist"]
struct Dist;

fn content_type(path: &str) -> &'static str {
    let extension = path.rsplit('.').next().unwrap_or_default();
    match extension {
        "html" => "text/html; charset=utf-8",
        "js" => "application/javascript",
        "wasm" => "application/wasm",
        "css" => "text/css",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

/// Serves the bundle on an ephemeral localhost port from a background thread
/// and returns the port. Localhost is a secure context, so WebGPU behaves
/// exactly as it does in a browser tab. Each window process serves its own
/// copy, so the windows share only the broker, not a page server.
fn serve_dist() -> u16 {
    let server = tiny_http::Server::http("127.0.0.1:0").expect("failed to bind localhost");
    let port = server
        .server_addr()
        .to_ip()
        .expect("expected an ip address")
        .port();
    std::thread::spawn(move || {
        for request in server.incoming_requests() {
            let path = request.url().split('?').next().unwrap_or("/");
            let path = path.trim_start_matches('/');
            let path = if path.is_empty() { "index.html" } else { path };
            match Dist::get(path) {
                Some(file) => {
                    let header = tiny_http::Header::from_bytes(
                        &b"Content-Type"[..],
                        content_type(path).as_bytes(),
                    )
                    .expect("static header is valid");
                    let response =
                        tiny_http::Response::from_data(file.data.into_owned()).with_header(header);
                    let _ = request.respond(response);
                }
                None => {
                    let _ = request.respond(tiny_http::Response::empty(404));
                }
            }
        }
    });
    port
}

struct App {
    url: String,
    title: String,
    #[cfg(feature = "networking")]
    shutdown: Option<network::ShutdownChannel>,
    window: Option<Window>,
    webview: Option<WebView>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(self.title.clone())
            .with_maximized(true);
        let window = event_loop
            .create_window(attributes)
            .expect("failed to create window");

        let builder = WebViewBuilder::new()
            .with_url(self.url.clone())
            .with_navigation_handler(|url| {
                url.starts_with("http://127.0.0.1") || url.starts_with("https://127.0.0.1")
            });
        #[cfg(feature = "agent")]
        let builder = builder.with_ipc_handler(|request| match request.body().as_str() {
            "enable-mcp" => agent::start(),
            "open-chat" => {
                agent::start();
                chat::start();
            }
            "enable-fs" => fs::start(),
            "enable-lsp" => lsp::start(),
            "enable-highlight" => highlight::start(),
            "enable-git" => git::start(),
            "enable-terminal" => pty::start(),
            _ => {}
        });
        #[cfg(target_os = "windows")]
        let builder = {
            use wry::WebViewBuilderExtWindows;
            builder.with_additional_browser_args("--enable-features=WebGPU")
        };
        let webview = builder.build(&window).expect("failed to create webview");

        self.window = Some(window);
        self.webview = Some(webview);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let WindowEvent::CloseRequested = event {
            #[cfg(feature = "networking")]
            if let Some(channel) = &self.shutdown {
                network::notify_shutdown(channel);
            }
            event_loop.exit();
        }
    }
}

fn main() {
    if Dist::get("index.html").is_none() {
        eprintln!("the web bundle is missing, build it first with `just dist`");
        std::process::exit(1);
    }
    let port = serve_dist();

    #[cfg(feature = "networking")]
    let (url, title, shutdown) = {
        let role = network::ShellRole::detect();
        let is_host = role.is_host();
        let shell_id = format!("neon-{}", std::process::id());
        let shutdown = network::start(role, shell_id.clone());
        let role_name = if is_host { "primary" } else { "child" };
        let title = if is_host { "Neon" } else { "Neon - Window" };
        let url = format!("http://127.0.0.1:{port}/?role={role_name}&shell={shell_id}");
        (url, title.to_string(), shutdown)
    };
    #[cfg(not(feature = "networking"))]
    let (url, title) = (format!("http://127.0.0.1:{port}/"), "Neon".to_string());

    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App {
        url,
        title,
        #[cfg(feature = "networking")]
        shutdown,
        window: None,
        webview: None,
    };
    event_loop.run_app(&mut app).expect("event loop failed");
}
