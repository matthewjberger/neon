# The Wire Protocol

Everything that crosses a context boundary is serde, defined once in the
`protocol` crate. Add a feature by adding a message and handling it on both
sides.

## The message pairs

- `ClientMessage` / `WorkerMessage`: page to and from the engine worker. Input,
  picking, plugin sync, command submission, console traffic, the command
  manifest, and a `Busy` flag the page renders as the top progress bar.
- `LangRequest` / `LangResponse`: page to and from the language worker.
  Vocabulary seeding and compile-check requests with keyed diagnostics.
- `AgentRequest` / `AgentResponse`: the agent surface, split editor-domain
  (answered by the page) and scene-domain (answered by the worker).
- `FsRequest` / `FsResponse`: page to and from the desktop filesystem bridge.
  Open folder, list directory, read and write files, project search.
- `LspClientMessage` / `LspServerMessage`: page to and from the language-server
  bridge, carrying framed LSP JSON-RPC plus server log lines.

## The reflective vocabulary

The worker derives the scene command vocabulary from the engine facade and ships
it to the page on `Ready` as a manifest (`CommandInfo` and a JSON schema) plus
the standard library source. The highlighter, the reference overlay, the
language worker, and the control panel all read from this. Add a free function to
`nightshade-api` and it becomes a `Command`, then it lights up across the editor
with no editor changes.

## Why one format

A single serde crate keeps the four contexts honest. The page never touches the
engine and the worker never touches the DOM. Everything they share is in
`protocol`, so a change that breaks a seam breaks the build, not the running app.
