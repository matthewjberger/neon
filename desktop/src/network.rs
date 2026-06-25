//! The native side of the multi-window shell contract. The first process hosts
//! the hearsay broker and its websocket listener and spawns more windows as
//! supervised child processes; a child joins the broker and exits when the host
//! publishes the shutdown topic or its own close topic. The page asks for a new
//! window over the webview IPC, which the host turns into a broker spawn, so any
//! window can open another.

use std::sync::OnceLock;

use nightshade::networking;

const BROKER_ADDRESS: &str = "127.0.0.1:8782";
const WEBSOCKET_ADDRESS: &str = "127.0.0.1:8783";
const SPAWN_TOPIC: &str = "neon/shell/request-spawn";
const SHUTDOWN_TOPIC: &str = "neon/shell/shutdown";

fn close_topic(shell_id: &str) -> String {
    format!("neon/shell/close-{shell_id}")
}

/// Whether this process hosts the broker (the first launch) or joined one a
/// host spawned, detected from the broker-address environment variable hearsay
/// sets on the children it launches.
#[derive(Clone)]
pub enum ShellRole {
    Host,
    Child { broker_address: String },
}

impl ShellRole {
    pub fn detect() -> Self {
        match std::env::var(networking::BROKER_ADDRESS_VARIABLE) {
            Ok(broker_address) => Self::Child { broker_address },
            Err(_) => Self::Host,
        }
    }

    pub fn is_host(&self) -> bool {
        matches!(self, Self::Host)
    }
}

/// The host's spawn channel: the network thread waits on it, the webview IPC
/// handler pushes onto it when the page asks for a new window.
static SPAWN: OnceLock<tokio::sync::mpsc::UnboundedSender<()>> = OnceLock::new();

/// Asks the host to open another window. A no-op on a child or before the
/// broker is up, so the page can call it unconditionally.
pub fn request_window() {
    if let Some(sender) = SPAWN.get() {
        let _ = sender.send(());
    }
}

/// Lets the main thread ask the network thread to broadcast the shutdown topic
/// and waits briefly for the broker to fan it out before the process exits, so
/// the children close with the host.
pub struct ShutdownChannel {
    sender: tokio::sync::mpsc::UnboundedSender<std::sync::mpsc::Sender<()>>,
}

pub fn notify_shutdown(channel: &ShutdownChannel) {
    let (acknowledge_sender, acknowledge_receiver) = std::sync::mpsc::channel();
    if channel.sender.send(acknowledge_sender).is_ok() {
        let _ = acknowledge_receiver.recv_timeout(std::time::Duration::from_millis(500));
    }
}

/// Runs the shell contract on a background thread. Returns the shutdown channel
/// for the host, `None` for a child or for a host whose broker port is already
/// taken (a second independent launch falls back to a lone window).
pub fn start(role: ShellRole, shell_id: String) -> Option<ShutdownChannel> {
    let is_host = role.is_host();
    let (channel_sender, channel_receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let Ok(runtime) = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        else {
            return;
        };
        runtime.block_on(async move {
            match role {
                ShellRole::Host => {
                    let Ok(broker) = networking::start_broker(BROKER_ADDRESS).await else {
                        return;
                    };
                    if let Err(error) =
                        networking::start_websocket_listener(&broker, WEBSOCKET_ADDRESS).await
                    {
                        eprintln!("failed to start the websocket listener: {error}");
                        return;
                    }
                    let Some(client) = connect_client(&shell_id, BROKER_ADDRESS).await else {
                        return;
                    };
                    let (shutdown_sender, shutdown_receiver) =
                        tokio::sync::mpsc::unbounded_channel();
                    let (spawn_sender, spawn_receiver) = tokio::sync::mpsc::unbounded_channel();
                    let _ = SPAWN.set(spawn_sender);
                    let _ = channel_sender.send(ShutdownChannel {
                        sender: shutdown_sender,
                    });
                    run_host(broker, client, shutdown_receiver, spawn_receiver).await;
                }
                ShellRole::Child { broker_address } => {
                    let Some(client) = connect_client(&shell_id, &broker_address).await else {
                        std::process::exit(0);
                    };
                    run_child(client, &shell_id).await;
                }
            }
        });
    });
    if is_host {
        channel_receiver.recv().ok()
    } else {
        None
    }
}

async fn connect_client(shell_id: &str, address: &str) -> Option<networking::Client> {
    let mut client = networking::create_client(shell_id, networking::ClientSettings::default());
    if networking::connect(&mut client, address).await.is_err() {
        return None;
    }
    Some(client)
}

async fn run_host(
    broker: networking::Broker,
    mut client: networking::Client,
    mut shutdown_receiver: tokio::sync::mpsc::UnboundedReceiver<std::sync::mpsc::Sender<()>>,
    mut spawn_receiver: tokio::sync::mpsc::UnboundedReceiver<()>,
) {
    if networking::subscribe(&mut client, &[SPAWN_TOPIC])
        .await
        .is_err()
    {
        return;
    }
    let mut window_counter: u32 = 0;
    loop {
        tokio::select! {
            message = networking::next_message(&mut client) => {
                let Some(message) = message else {
                    break;
                };
                if message.topic == SPAWN_TOPIC {
                    spawn_window(&broker, &mut window_counter).await;
                }
            }
            request = spawn_receiver.recv() => {
                if request.is_none() {
                    break;
                }
                spawn_window(&broker, &mut window_counter).await;
            }
            acknowledge = shutdown_receiver.recv() => {
                let Some(acknowledge) = acknowledge else {
                    break;
                };
                let _ = networking::publish_json(
                    &client,
                    SHUTDOWN_TOPIC,
                    "{}",
                    networking::Route::Global,
                )
                .await;
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                let _ = acknowledge.send(());
                break;
            }
        }
    }
}

async fn spawn_window(broker: &networking::Broker, window_counter: &mut u32) {
    let Ok(executable) = std::env::current_exe() else {
        return;
    };
    *window_counter += 1;
    let _ = networking::spawn_app(
        broker,
        networking::App {
            name: format!("neon-window-{window_counter}"),
            path: executable.display().to_string(),
            restart_policy: networking::RestartPolicy::Never,
            ..Default::default()
        },
    )
    .await;
}

async fn run_child(mut client: networking::Client, shell_id: &str) {
    let close = close_topic(shell_id);
    if networking::subscribe(&mut client, &[SHUTDOWN_TOPIC, close.as_str()])
        .await
        .is_err()
    {
        std::process::exit(0);
    }
    while let Some(message) = networking::next_message(&mut client).await {
        if message.topic == SHUTDOWN_TOPIC || message.topic == close {
            std::process::exit(0);
        }
    }
    std::process::exit(0);
}
