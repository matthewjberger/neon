# Neon design

Neon is a plugin-first 3D editor. The UI is Leptos in Rust, the nightshade
engine runs in a web worker, and you author plugins in rhai that produce
`Command` and consume `Event`. The whole stack is Rust plus a few lines of wasm
bootstrap JavaScript. No npm, no bundler, no JavaScript framework.

This document is the architecture and the decisions, including the layers that
are designed and scaffolded but not yet fully built. It is the source of truth
as the product grows.

## Principles

- **All Rust, no npm.** Every dependency is a Rust crate. The only JavaScript is
  the per-worker bootstrap (`runtime/worker.js`, `runtime/lang_worker.js`), a few
  lines each, with no packages. This rules out CodeMirror and tree-sitter (npm
  and C respectively); highlighting is a hand-written rhai scanner in Rust plus
  the command manifest, which for a single language is more accurate than a
  generic grammar and adds no dependency.
- **Data-oriented, not OOP.** State is a `Copy` struct of signals; behavior is
  free functions; components are plain functions. Nothing is an object that owns
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
  bundle, and (milestone 2) hosts the Claude MCP bridge and chat relay.

## The plugin model

A scene plugin is a `PluginSource` (`protocol`): id, name, rhai source, enabled.
The page owns the set, persists it to local storage (`src/plugins.rs`), and syncs
the whole list to the worker on every change. The worker (`worker/src/systems/
plugins.rs`) rebuilds the engine's global scripts from the enabled plugins,
prepends the standard library so its helpers are in scope, and runs everything
each tick through `run_scripts`. Commands a plugin produces apply as one deferred
batch; the tick's traffic streams back to the console.

Inside a plugin, `commands` is the array you push api `Command`s to, `events` is
this frame's `Event`s, and the runtime also exposes `dt`, `time`, `keys`,
`mouse`, `named`, `tagged`, and `replies`. The standard library
(`worker/stdlib/*.rhai`) adds higher-level builders called as methods on
`commands` and filters on `events`.

## The standard library

Procedural helpers, embedded from `worker/stdlib/`:

- `shapes` — `commands.cube`, `sphere`, `glowing`, `grid`, `ring`.
- `color` — `hsv`, `gray`, `mix_color`, and `RED`/`GREEN`/`BLUE`/`WHITE`.
- `motion` — `commands.spin`, `bob`, `orbit` (take `dt`/`time` explicitly, since
  rhai functions do not capture scope).
- `events` — `events.hits`, `sensor_hits`, `other`.
- `input` — `axis_x`, `axis_z`, `held`.
- `random` — `random_color`, `random_point`, `random_pick`.

The library is sent to the page on `Ready` so the editor can show its source and
the language service can offer its helpers.

## The editor surface

A native textarea for editing (caret, IME, clipboard, accessibility all free)
with a Rust highlight `<pre>` layer behind it sharing the same box
(`src/components/editor_pane.rs`). Highlighting comes from `src/highlight.rs`, a
hand-written rhai scanner; command tokens are colored from the manifest, so the
color set never drifts from what a script can call. Edits update the plugin
source, persist, and after a short pause sync the scene and ask the language
worker to compile-check. Diagnostics show in a strip under the editor.

The language service is reflective: the worker derives the command vocabulary
from `command_manifest`/`command_schema`, and the reference, highlighter, and
language worker all read from it. Add a function to the facade and it becomes a
`Command`, then it lights up across the editor with no editor changes.

## Theming

`data-theme` on the document root selects a theme; CSS variables under
`[data-theme="..."]` in `public/styles.css` define each. Four ship (midnight,
ember, forest, paper). The choice persists in local storage. The toolbar picker
sets `state.theme`, an effect applies and saves it.

## Claude / MCP integration (milestone 2, scaffolded)

The toolbar has the Claude toggle. The chat pane (`src/components/chat.rs`) opens
it, asks the desktop shell over webview IPC to start the bridge, and connects to
the chat relay websocket, rendering the Claude subprocess's stream-json.

The agent surface is in `protocol` as `AgentRequest`/`AgentResponse`. It spans
two domains, because neon's state is split:

- **Editor domain (answered by the page):** `GetEditorState`, `GetBuffer`,
  `SetBuffer`, `ListPlugins`, `EditPlugin`. Reading and writing what the user
  sees and edits.
- **Scene domain (answered by the engine worker):** `RunCommand`, `QueryScene`,
  `Screenshot`. The worker handler is `worker/src/lib.rs::handle_agent`.

The desktop shell (`desktop/`) hosts the MCP server that exposes these as tools
to Claude and the relay that pipes a `claude` subprocess. That host process is
the remaining piece of this milestone. The page side (relay client, request
routing page-vs-worker) is the next code to write; the contract is already in
`protocol`. This mirrors the nightshade editor's proven relay/bridge pattern.

## The editor API and editor plugins (milestone 3, designed)

A second plugin kind: editor plugins that extend the editor itself, not the
scene. They run in a neon-owned rhai engine on the page (for latency, since they
touch the editor synchronously) with an Editor API neon registers:

- **Buffers:** read/insert/delete text, cursor and selection, multiple buffers.
- **Windows, panels, tiles:** open and arrange panes; a tiling layout the API
  drives.
- **Terminals:** spawn and write to integrated terminals (desktop only).
- **Keymaps:** bind keys per mode, register commands and menu items, notify and
  set status.

This is what carries the vim keybindings layer: vim is an editor plugin that maps
keys per mode and drives the buffer through the Editor API. The nightshade
`Command`/`Event` bus is closed (engine-defined, no custom-emit), so the editor
API is necessarily a neon layer on top, not a ride on that bus. The two plugin
kinds share authoring (rhai, hooks, viewable source), so it stays one experience.

## Milestones

1. **Scene-plugin core (this build).** Workspace, protocol, engine-worker scene
   scripting, standard library, the Leptos UI (editor, plugin manager, console,
   reference, viewport, toolbar, theming), and the language worker. Author rhai
   plugins in-app and watch them drive the 3D scene live.
2. **Claude / MCP.** The desktop MCP bridge and chat relay, the page relay client,
   and agent-request routing across page and worker.
3. **Editor API and vim.** The page-side editor-plugin runtime, the Editor API
   (buffers, windows, panels, tiles, terminals, keymaps), and the vim layer.

## Build

```sh
just run       # native webview over the bundle
just run-web   # serve in the browser
```

`just workers` builds the engine and language wasm into `runtime/`; `just build`
adds the trunk bundle. Path dependencies point at a sibling `../nightshade`
checkout for the live facade.
