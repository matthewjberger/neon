//! Shared plumbing for the desktop bridges' page sockets: a bounded outgoing
//! queue and the task that drains it into the socket. Bounding the queue makes a
//! bridge backpressure on a slow page instead of buffering messages without
//! limit, and the one writer task replaces the copy each bridge used to spawn.

use futures_util::SinkExt;
use futures_util::stream::SplitSink;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

/// The most page-bound messages a bridge buffers before a send awaits the
/// socket (or, from a blocking thread, parks). A slow page throttles the
/// producer here rather than growing memory without bound.
pub const PAGE_QUEUE: usize = 256;

/// The outgoing half of a bridge's page socket.
pub type PageSink = SplitSink<WebSocketStream<TcpStream>, Message>;

/// Spawns the task that forwards queued text frames to the page until the queue
/// closes or the socket errors. The returned handle is aborted when the
/// connection ends.
pub fn spawn_writer(mut sink: PageSink, mut queue: mpsc::Receiver<String>) -> JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(text) = queue.recv().await {
            if sink.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    })
}
