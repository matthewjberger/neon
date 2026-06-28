# Neon design

Neon is a code editor written in Rust. The UI is Leptos, it edits files from disk
with rust-analyzer for Rust projects, and it is extensible through rhai plugins:
editor plugins add editor functionality (keybindings, modal editing, commands),
and scene plugins drive a live 3D view rendered by the nightshade engine in a web
worker. The whole stack is Rust plus a few lines of wasm bootstrap JavaScript. No
npm, no bundler, no JavaScript framework.

This document is the architecture and the decisions. It is the source of truth as
the product grows.

## Principles

- **All Rust, no npm.** Every dependency is a Rust crate. The only JavaScript is
  the per-worker bootstrap (`runtime/worker.js`, `runtime/lang_worker.js`), a few
  lines each, with no packages. This rules out CodeMirror (npm), and keeps the
  page's wasm free of C: tree-sitter and its grammars are C, so they run natively
  in the desktop shell (`desktop/src/highlight.rs`, all Rust crates that bundle
  their parser) and reach the page over a websocket bridge, the same shape as the
  rust-analyzer and filesystem bridges. The page asks the shell to parse a buffer
  and paints the token spans it gets back. Until they arrive, in a plain browser
  with no shell, and for rhai, it falls back to a hand-written multi-language
  scanner in Rust (`src/highlight.rs`) that colors rhai commands from the
  manifest; richer language intelligence comes from LSP.
- **Data-oriented, not OOP.** State is a `Copy` struct of signals, behavior is
  free functions, components are plain functions. Nothing is an object that owns
  the app, the engine, or the workers.
- **One wire format.** Every cross-context message is serde in `protocol/`.
- **Plugins easy and plentiful.** A plugin is a small rhai file with `on_start`
  and `on_tick`. The standard library does the heavy lifting so a useful plugin
  is a few lines. Source for every plugin and the standard library is visible in
  the app.

## Contexts

Four isolated contexts, each doing what it is best at.

- **Main thread (`src/`, the `neon` crate):** the Leptos UI. The code editor,
  plugin manager, console, reference, viewport host, toolbar, and chat. No
  engine, no rhai, no npm.
- **Engine worker (`worker/`):** the `nightshade-api` facade plus the offscreen
  renderer. Runs the scene plugins each tick with `run_scripts`, applies the
  `Command`s they produce, renders, and exports the command manifest and the
  standard library to the page.
- **Language worker (`lang/`):** links only `rhai`. Compile-checks plugin source
  and flags unknown command calls, off the render thread.
- **Desktop shell (`desktop/`):** a `wry` webview that serves and embeds the web
  bundle, and hosts the bridges the page cannot run itself: the Claude MCP bridge
  and chat relay, a filesystem bridge (disk access plus project search), and a
  language-server bridge that spawns rust-analyzer and frames LSP over stdio.

## The plugin model

A scene plugin is a `PluginSource` (`protocol`): id, name, rhai source, enabled.
The page owns the set, persists it to local storage (`src/plugins.rs`), and syncs
the whole list to the worker on every change. The worker (`worker/src/systems/
plugins.rs`) rebuilds the engine's global scripts from the enabled plugins,
prepends the standard library so its helpers are in scope, and runs everything
each tick through `run_scripts`. Commands a plugin produces apply as one deferred
batch, and the tick's traffic streams back to the console.

Inside a plugin, `commands` is the array you push api `Command`s to, `events` is
this frame's `Event`s, and the runtime also exposes `dt`, `time`, `keys`,
`mouse`, `named`, `tagged`, and `replies`. The standard library
(`worker/stdlib/*.rhai`) adds higher-level builders called as methods on
`commands` and filters on `events`.

## The standard library

Procedural helpers, embedded from `worker/stdlib/`:

- `shapes`: `commands.cube`, `sphere`, `glowing`, `grid`, `ring`.
- `color`: `hsv`, `gray`, `mix_color`, and `RED`/`GREEN`/`BLUE`/`WHITE`.
- `motion`: `commands.spin`, `bob`, `orbit` (take `dt`/`time` explicitly, since
  rhai functions do not capture scope).
- `events`: `events.hits`, `sensor_hits`, `other`.
- `input`: `axis_x`, `axis_z`, `held`.
- `random`: `random_color`, `random_point`, `random_pick`.

The library is sent to the page on `Ready` so the editor can show its source and
the language service can offer its helpers.

## The editor surface

A native textarea for editing (caret, IME, clipboard, accessibility all free)
with a Rust highlight `<pre>` layer behind it sharing the same box, a line-number
gutter, and a tab strip (`src/components/editor_pane.rs`). A pane holds many
buffers as tabs; panes split right or below and resize. Highlighting comes from
`src/highlight.rs`, a hand-written multi-language scanner (rust, toml, json,
javascript, rhai), with rhai commands colored from the manifest.

Every edit goes through `state.set_buffer_text`, which records the pre-edit text
into a per-buffer undo stack (`src/undo.rs`) before writing, so `Ctrl+Z`/`Ctrl+Y`
undo every path (typing, plugin ops, find, completion) even though native
textarea undo is bypassed by programmatic edits. Find and replace (`src/find.rs`)
act on the focused textarea; project-wide search runs on the desktop. Editing a
scene plugin syncs the scene and asks the language worker to compile-check;
diagnostics show in a strip under the editor.

The rhai language service is reflective: the worker derives the command
vocabulary from `command_manifest`/`command_schema`, and the reference,
highlighter, and language worker all read from it. Add a function to the facade
and it becomes a `Command`, then it lights up across the editor with no editor
changes.

## Files and rust-analyzer

A pane buffer can be a file from disk, not only a plugin. The filesystem bridge
opens a folder and a lazily loaded tree, reads and writes files, and searches the
workspace (respecting gitignore); the open folder and files are restored on
launch (`src/session.rs`). For Rust files the page is an LSP client
(`src/lsp.rs`, one `Client` struct): after a consent prompt it starts
rust-analyzer (discovered through rustup), runs the initialize handshake, syncs
open files with `didOpen`/`didChange`, and surfaces diagnostics, completion,
hover, signature help, go to definition, references, document symbols, rename,
code actions, formatting, and diagnostic stepping. Every feature is an editor
command, so the palette, the Spacemacs leader, and editor plugins all reach the
same set. The LSP log panel shows the server output.

## Theming

`data-theme` on the document root selects a theme. CSS variables under
`[data-theme="..."]` in `public/styles.css` define each. Five ship: VS Code Dark
(the default, for familiarity), midnight, ember, forest, and paper. The choice
persists in local storage. The toolbar picker sets `state.theme`, an effect
applies and saves it.

## Claude / MCP integration

The toolbar has the Claude toggle. The chat pane (`src/components/chat.rs`) opens
it, asks the desktop shell over webview IPC to start the bridge, and connects to
the chat relay websocket, rendering the Claude subprocess's stream-json.

The agent surface is in `protocol` as `AgentRequest`/`AgentResponse`. It spans
two domains, because neon's state is split:

- **Editor domain (answered by the page):** `GetEditorState`, `GetBuffer`,
  `SetBuffer`, `ListPlugins`, `EditPlugin`, plus `GetApiReference` (the command
  and helper vocabulary) and `GetConsole`, so the agent learns the API and sees
  runtime errors instead of probing. Edits return compile diagnostics.
- **Scene domain (answered by the engine worker):** `RunCommand`, `QueryScene`,
  `Screenshot` (a real PNG capture of the viewport). The worker handler is
  `worker/src/lib.rs::handle_agent`.

This is fully wired: the page relay client (`src/relay.rs`) routes each request
to its owner, and the desktop shell (`desktop/src/agent.rs`) hosts the MCP HTTP
server that exposes these as tools and the chat relay that pipes a `claude`
subprocess pointed at it. It mirrors the nightshade editor's relay and bridge
pattern.

## The editor API and editor plugins

A second plugin kind: editor plugins that extend the editor itself, not the
scene. They run in a neon-owned rhai engine on the page (for latency, since they
touch the editor synchronously). An editor plugin's `on_key()` reads the keystroke
and a persistent `state` map and pushes ops the host applies:

- **Buffer and cursor:** the host owns the text, so the ops do real editing:
  caret motion (character, line, word, smart line start, find char), edits
  (insert, delete char/word/line, kill to line end, duplicate line, move line up
  or down, join, indent, outdent, toggle comment), and mode and status.
- **Editor commands:** `RunCommand` runs any entry in the editor-command registry
  (`src/commands.rs`): split and focus panes, toggle the panels, switch the
  sidebar, run or pause, reset, cycle themes, open buffers, open the palette or
  help. `OpenPalette` opens the command palette directly.
- **Which-key menu:** `ShowMenu`/`HideMenu` publish the leader menu, the bottom
  panel that lists the next keys and what they do. The keymap and its menu live
  together in the plugin.

The default plugin is a Spacemacs layer: vim modal editing plus an `SPC` leader
whose which-key menu carries window management and avy-style jumps (`SPC j w`
labels every on-screen word over a dimmed buffer, `SPC j l` every line, `SPC j j`
every occurrence of a typed character, and the caret jumps to the label you
type). The catalog ships many more as
opt-in: a Vim and an Emacs layer, auto pairs, better escape, comment toggle and
the gcc comment object, line tools, word delete, join lines, smart home, jump to
char, blank lines, and move lines, plus a stack of scene-plugin examples. The nightshade `Command`/`Event` bus is closed (engine-defined, no
custom-emit), so the editor API is a neon layer on top, not a ride on that bus.
The two plugin kinds share authoring (rhai, hooks, viewable source), so it stays
one experience.

## Milestones

1. **Scene-plugin core.** Workspace, protocol, engine-worker scene scripting,
   standard library, the Leptos UI (editor, plugin manager, console, reference,
   viewport, toolbar, theming), and the language worker. Author rhai plugins
   in-app and watch them drive the 3D scene live. Built.
2. **Editor API and editor plugins.** The page-side editor-plugin runtime, the
   editor-command registry, the command palette, split panes, and the Spacemacs
   and Vim layers with the which-key leader menu. Built.
3. **Claude / MCP.** The chat pane, the page relay client, the agent-request
   routing, and the desktop MCP server with viewport screenshot are all in place.
   Built.
4. **Files and rust-analyzer.** The filesystem bridge (tree, read/write, project
   search), file buffers with tabs, session restore, and the rust-analyzer LSP
   client (diagnostics, completion, hover, signature help, definition,
   references, symbols, rename, code actions, formatting) behind a consent
   prompt. Built.
5. **Deeper editor surface.** A custom-rendered surface for multi-cursor,
   integrated terminals, and richer multi-buffer manipulation, extending the same
   op and command vocabulary. Undo, tabs, find, and project search are already in.

## Build

```sh
just run       # native webview over the bundle
just run-web   # serve in the browser
```

`just workers` builds the engine and language wasm into `runtime/`. `just build`
adds the trunk bundle. Path dependencies point at a sibling `../nightshade`
checkout for the live facade.
