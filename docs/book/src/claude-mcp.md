# Claude and MCP

Neon embeds Claude through MCP, so an agent can read editor state, edit buffers,
and drive the scene, all through the same surfaces you use.

## The chat pane

The Claude toggle in the top bar opens the chat pane. It asks the desktop shell
over webview IPC to start the bridge, connects to the chat relay websocket, and
renders the Claude subprocess's stream-json output.

## The agent surface

The agent surface is in `protocol` as `AgentRequest` and `AgentResponse`. It
spans two domains, because neon's state is split:

- **Editor domain, answered by the page.** `GetEditorState`, `GetBuffer`,
  `SetBuffer`, `ListPlugins`, `EditPlugin`, plus `GetApiReference` (the command
  and helper vocabulary) and `GetConsole`, so the agent learns the API and sees
  runtime errors instead of probing. Edits return compile diagnostics.
- **Scene domain, answered by the engine worker.** `RunCommand`, `QueryScene`,
  and `Screenshot`, a real PNG capture of the viewport.

## How it routes

The page relay client (`src/relay.rs`) routes each request to its owner: editor
requests it answers itself, scene requests it forwards to the worker. The desktop
shell (`desktop/src/agent.rs`) hosts the MCP HTTP server that exposes these as
tools, and the chat relay that pipes a `claude` subprocess pointed at it.

So an agent can read the buffer, propose an edit, get the diagnostics back,
query or command the scene, and screenshot the result, without leaving the
editor.
