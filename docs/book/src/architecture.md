# Architecture

This chapter is the map of the codebase. For the high-level shape, read
[The Four Contexts](contexts.md) first. This goes a layer deeper, file by file.

## The page (`src/`, the `neon` crate)

The Leptos UI, data-oriented throughout.

- `state.rs` is the whole page state as a `Copy` struct of signals. Behavior is
  free functions, not methods on an object.
- `app.rs` composes the components and wires the global handlers.
- `components/` holds the surfaces: `editor_pane`, `toolbar`, `palette`,
  `file_tree`, `search`, `console`, `control_panel`, `context_menu`, `lsp_menus`,
  `lsp_panel`, `popups`, `which_key`, `reference`, `chat`, and more.
- `commands.rs` is the one command registry the palette, leader, menus, and
  plugins all drive.
- `editor_plugins.rs` runs the page-side rhai engine and applies the ops.
- `lsp.rs`, `fs.rs`, `relay.rs`, `bridge.rs` are the page sides of the four
  bridges and the worker.
- `highlight.rs`, `undo.rs`, `find.rs`, `jump.rs`, `caret.rs`, `session.rs`,
  `theme.rs`, `plugins.rs` are the editing-surface support.

## The engine worker (`worker/`)

The `nightshade-api` facade plus the offscreen renderer. `src/lib.rs` owns the
canvas and the frame loop and answers scene-domain agent requests.
`worker/stdlib/` is the scene standard library, embedded and prepended to every
plugin.

## The language worker (`lang/`)

Links only rhai. It compile-checks plugin source and validates command calls
against the seeded vocabulary, off the render thread.

## The desktop shell (`desktop/`)

A `wry` webview that serves and embeds the bundle, plus the four relays:
`agent.rs` (MCP server and chat relay), `fs.rs` (filesystem and search), and
`lsp.rs` (the rust-analyzer bridge).

## The protocol (`protocol/`)

The wire format for every seam, the one crate both sides of each bridge share.
See [The Wire Protocol](protocol.md).

## Adding a feature

The shape is always the same: add a message to `protocol`, handle it on each
side, and surface it in `components/` and `commands.rs`. A new scene command is
even cheaper: add a free function to `nightshade-api` and the reflective
vocabulary carries it across the editor.
