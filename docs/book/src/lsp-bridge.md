# How the LSP Bridge Works

Two halves: the desktop bridge that owns the rust-analyzer process, and the page
client that speaks LSP.

## The desktop bridge

`desktop/src/lsp.rs` is a websocket relay. It discovers rust-analyzer through
`rustup`, spawns it, and shuttles LSP JSON-RPC between the server and the page.
The server speaks `Content-Length` framed messages over stdio, so the bridge
frames outgoing messages and reframes incoming stdout into whole JSON payloads.
It also streams the server's stderr to the page as log lines.

## The page client

`src/lsp.rs` is the client, and all its state lives in one `Client` struct: the
socket, the handshake flag, the request-id counter, the per-file document
versions, the latest diagnostics, the in-flight requests, and the pending edits
for files not yet open. After consent it asks the desktop to start the server,
runs the `initialize` handshake, and then drives every feature as an editor
command.

## Reopen on restart

A fresh server starts with no open documents. So when the client sees a server
announce itself, it clears its open-document set and reopens every file. Without
that, a reconnect would leave the client believing files were open while the new
process had never seen them, and edits would be rejected.

## Applying edits

Format, rename, and code actions all return ranged edits. One shared applier
resolves each edit's line and character to a unit offset and splices from the end
so earlier offsets stay valid. For a file that is open it patches the buffer and
sends `didChange`. For a file that is not open it stores the edits, opens the
file, and applies them once the content arrives. The same applier handles the
server's `workspace/applyEdit` request, which is how command-driven code actions
land their changes.

## Two checkers, side by side

rust-analyzer drives diagnostics for file buffers. The rhai language worker drives
diagnostics for scene plugins. They are parallel paths into the same diagnostics
strip, picked by what the focused buffer is.
