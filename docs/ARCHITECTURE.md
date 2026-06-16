# Neon architecture

Neon is a code editor written in Rust. The UI is Leptos, it edits files from disk
with rust-analyzer for Rust projects, and it is extensible through rhai plugins:
editor plugins add editor functionality (keybindings, modal editing, commands),
and scene plugins drive a live 3D view rendered by the nightshade engine in a web
worker. The whole stack is Rust plus a few lines of wasm bootstrap JavaScript. No
npm, no bundler, no JavaScript framework.

## Contexts

Four isolated execution contexts, each doing what it is best at. None can block
the others.

```
                         main thread (neon, Leptos)
            code editor | plugin manager | console | reference
                   toolbar | viewport host | Claude chat
                  |              |                    |
   ClientMessage  |  LangRequest |        AgentRequest
   WorkerMessage  |  LangResponse|        AgentResponse
                  v              v                    v
        engine worker     language worker      desktop shell
        (nightshade-api)  (rhai only)          (wry webview,
        run_scripts,      compile-check,        MCP bridge,
        render, agent     validate              chat relay)
```

- **Main thread (`src/`, the `neon` crate).** The Leptos UI. Data-oriented: state
  is a `Copy` struct of signals (`state.rs`), behavior is free functions,
  components are plain `#[component]` functions. No engine, no rhai, no npm.
- **Engine worker (`worker/`).** The `nightshade-api` facade plus the offscreen
  renderer. Runs scene plugins through `run_scripts` each tick, applies the
  `Command`s they produce, renders, and answers scene-domain agent requests. The
  only place that touches the engine.
- **Language worker (`lang/`).** Links only `rhai`. Compile-checks plugin source
  and flags unknown command calls off the render thread.
- **Desktop shell (`desktop/`).** A `wry` webview that serves and embeds the web
  bundle, and hosts four relays: the Claude MCP bridge and chat relay, a
  filesystem bridge (disk access for the page through `rfd` and `tokio::fs`), and
  a language-server bridge that discovers rust-analyzer through rustup, spawns it,
  and frames LSP JSON-RPC over stdio.

The only JavaScript is `runtime/worker.js` and `runtime/lang_worker.js`, a few
lines each that boot the respective wasm modules and buffer early messages. No
packages.

## Wire format

Everything that crosses a context boundary is serde, defined once in
`protocol/`:

- `ClientMessage` / `WorkerMessage`: page to and from the engine worker:
  input, picking, plugin sync, command submission, console traffic, the manifest,
  and a `Busy` flag the page renders as the top progress bar.
- `LangRequest` / `LangResponse`: page to and from the language worker:
  vocabulary seeding and compile-check requests with keyed diagnostics.
- `AgentRequest` / `AgentResponse`: the agent surface, split editor-domain
  (answered by the page) and scene-domain (answered by the worker).
- `FsRequest` / `FsResponse`: page to and from the desktop filesystem bridge:
  open folder, list directory, read and write files, project search.
- `LspClientMessage` / `LspServerMessage`: page to and from the language-server
  bridge, carrying framed LSP JSON-RPC plus server log lines.

## Data flow: a frame and an edit

**Startup.** The viewport transfers its canvas to the engine worker and sends
`Init`. The worker builds the renderer, runs the offscreen frame loop, and posts
`Ready` carrying the command manifest, the command json schema, and the standard
library. The page stores these, seeds the language worker, and syncs the plugin
set to the worker. Until `Ready` arrives the page shows the startup card and the
top progress bar, and the worker posts `Busy` around each scene rebuild so the
bar reappears while a plugin sync or reset replays.

**Each engine frame.** `tick_offscreen` runs the engine schedule and then
`Scene::run_systems`, which runs the camera controllers and `plugins::tick`. That
clears the immediate-draw pools, runs every enabled plugin through `run_scripts`,
applies the `Command`s they produced as one deferred batch, and buffers the
tick's commands, events, and errors. The render loop drains that buffer to the
page as a `Report` and posts stats.

**An edit.** Typing updates the active plugin's source signal. After a short
debounce the page sends the whole plugin set to the worker (`SetPlugins`) and the
source to the language worker (`Check`). The worker replays the plugins from a
clean stage, so editing or toggling never leaves a prior run's entities behind.
The language worker returns diagnostics, shown under the editor.

## The plugin model

A plugin is a `PluginSource`: id, name, rhai source, enabled flag. The page owns
the set, persists it to local storage (`plugins.rs`), and is the single authority
that syncs it to the worker. The worker rebuilds the engine's global scripts from
the enabled plugins, prepends the standard library so its helpers are in scope,
and runs everything through `run_scripts`.

Inside a plugin, `on_start` runs once and `on_tick` every frame. `commands` is
the array you push api `Command`s to, `events` is this frame's `Event`s, and the
runtime exposes `dt`, `time`, `keys`, `mouse`, `named`, `tagged`, and `replies`.

## The standard library

Procedural helpers in `worker/stdlib/*.rhai`, embedded into the worker and
prepended to every plugin. Called as methods on `commands` and `events` or as
free functions:

- `shapes`: `cube`, `sphere`, `glowing`, `grid`, `ring`.
- `color`: `hsv`, `gray`, `mix_color`, and named colors.
- `motion`: `spin`, `bob`, `orbit`.
- `events`: `hits`, `sensor_hits`, `other`.
- `input`: `axis_x`, `axis_z`, `held`.
- `random`: `random_color`, `random_point`, `random_pick`.

The library is sent to the page on `Ready` so the editor shows its source and the
language service offers its helpers.

## The editor surface

The code editor (`components/editor_pane.rs`) is a native textarea with a Rust
highlight `<pre>` layer behind it sharing the same box, a line-number gutter, and
a tab strip. A pane holds many buffers as tabs; panes split (right or below) and
resize, each editing independently. Highlighting is a hand-written multi-language
scanner (`highlight.rs`): per-language keyword and comment rules for rust, toml,
json, javascript, and rhai, with rhai command tokens colored from the manifest.

Edits funnel through `state.set_buffer_text`, which records the pre-edit text into
a per-buffer undo stack (`undo.rs`) before writing, so `Ctrl+Z`/`Ctrl+Y` undo
every edit path (typing, plugin ops, find, completion) even though native
textarea undo is bypassed by programmatic edits. Find and replace (`find.rs`,
`Ctrl+F`) act on the focused textarea. Project-wide search runs on the desktop
and jumps to a line.

The rhai language service is reflective. The worker derives the command
vocabulary from `command_manifest` and `command_schema`, and the highlighter, the
reference overlay, and the language worker all read from it. Add a free function
to `nightshade-api` and it becomes a `Command`, then it lights up across the
editor with no editor changes.

## Theming

`data-theme` on the document root selects a theme. CSS variable blocks under
`[data-theme="..."]` in `public/styles.css` define each. The id persists in local
storage (`theme.rs`). The toolbar picker sets `state.theme`; an effect applies
and persists it.

## Claude / MCP integration

The toolbar Claude toggle opens the chat pane (`components/chat.rs`), which asks
the desktop shell over webview IPC to start the bridge and connects to the chat
relay websocket, rendering the Claude subprocess's stream-json. The agent surface
(`AgentRequest`/`AgentResponse`) spans the editor domain (buffers, panels,
plugins, answered by the page) and the scene domain (entities, screenshot,
answered by the worker). The desktop-hosted MCP server that exposes these as
tools and pipes the `claude` subprocess is the remaining piece.

## The editor-manipulation API and vim

A second plugin kind, editor plugins, runs in a page-side rhai engine
(`src/editor_plugins.rs`). An editor plugin's `on_key()` reads `key`, `mode`,
`ctrl`/`shift`/`alt`, and a persistent `state` map, and pushes ops the host
applies: `Consume`, `SetMode`, `SetStatus`, `Insert`, `Move`, `MoveLine`,
`LineStart`/`LineEnd`, `NextWord`/`PrevWord`, `DeleteForward`/`DeleteBackward`,
`DeleteLine`, plus the ops that drive the editor itself: `RunCommand` (run a
named editor command), `OpenPalette`, and `ShowMenu`/`HideMenu` (publish the
which-key menu). The dispatch mirrors the scene-plugin model, on the editor
instead of the scene, and runs synchronously in the keydown handler so modal
editing has no latency.

Editor commands live in one registry (`src/commands.rs`): split and focus panes,
toggle the panels, switch the sidebar view, run or pause, reset, cycle themes,
open buffers, and open the palette or help. The command palette and the leader
menus both drive this one set, and an editor plugin invokes any of it through
`RunCommand`, so plugins dictate what the editor does, not just what the buffer
holds.

The default editor plugin is a Spacemacs layer: vim modal editing plus an `SPC`
leader. Pressing `SPC` publishes a which-key menu through `ShowMenu`, and each
prefix narrows it (`SPC w` for windows, `SPC t` for toggles, and so on). A Vim
layer and an editor-plugin template ship alongside it. All three are editable and
toggleable in the plugin panel, so the bindings and the menus are tuned by
editing rhai, live. nightshade's `Command`/`Event` bus is closed, so this is a
neon layer on top, sharing the rhai authoring experience.

The buffer, cursor, command, tab, split-pane, and undo surface is implemented on
the textarea. A custom-rendered surface (multi-cursor) and integrated terminals
extend the same op and command vocabulary as that UI grows.

## Files and rust-analyzer

Buffers are not only plugins. A pane can show a file opened from disk through the
filesystem bridge: open a folder, browse the lazily loaded tree, edit, and save.
A file buffer carries its path, text, and dirty flag in `state.files`, and the
status bar shows its language by extension. The opened folder and files are saved
to local storage and reopened on launch (`session.rs`).

For a Rust file the page acts as an LSP client (`src/lsp.rs`), whose state lives
in one `Client` struct. After a consent prompt (spawning a process), it asks the
desktop to start rust-analyzer, runs the initialize handshake, syncs open files
with `didOpen` and `didChange` (the overlay model, so the server sees unsaved
edits), and turns `publishDiagnostics` into the diagnostics strip. It also
requests completion at the caret (a popup anchored with a canvas-measured font
advance) and hover under the pointer. The LSP log panel shows the server's
output. The rhai language worker still drives plugin diagnostics; rust-analyzer
is the parallel path for file buffers.

## Project search and the filesystem bridge

The page has no disk access, so the filesystem bridge (`desktop/src/fs.rs`, a
websocket relay) runs every file operation natively: the folder picker, directory
listing, file read and write, and a project search that walks the workspace with
the `ignore` crate so it respects gitignore. The page client (`src/fs.rs`) sends
`FsRequest`s and applies each `FsResponse` to the tree, the open file buffers, and
the search results.

## Build

```sh
just run       # native webview over the bundle
just run-web   # serve in the browser
```

`just workers` builds the engine and language wasm into `runtime/` via
wasm-bindgen and wasm-opt; `just build` adds the trunk bundle. Path dependencies
point at a sibling `../nightshade` checkout for the live facade.
