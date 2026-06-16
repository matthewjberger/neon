# Neon architecture

Neon is a plugin-first 3D editor. The UI is Leptos in Rust, the nightshade
engine runs in a web worker, and plugins are rhai scripts that produce `Command`
and consume `Event`. The whole stack is Rust plus a few lines of wasm bootstrap
JavaScript. No npm, no bundler, no JavaScript framework.

## Contexts

Four isolated execution contexts, each doing what it is best at. None can block
the others.

```
                         main thread (neon, Leptos)
            code editor | plugin manager | console | reference
                   toolbar | viewport host | Claude chat
                  |              |                    |
   ClientMessage  |  LangRequest |        AgentRequest (milestone 2)
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
  bundle, and hosts the Claude MCP bridge and chat relay (milestone 2).

The only JavaScript is `runtime/worker.js` and `runtime/lang_worker.js`, a few
lines each that boot the respective wasm modules and buffer early messages. No
packages.

## Wire format

Everything that crosses a context boundary is serde, defined once in
`protocol/`:

- `ClientMessage` / `WorkerMessage` — page to and from the engine worker:
  input, picking, plugin sync, command submission, console traffic, the manifest.
- `LangRequest` / `LangResponse` — page to and from the language worker:
  vocabulary seeding and compile-check requests with keyed diagnostics.
- `AgentRequest` / `AgentResponse` — the agent surface, split editor-domain
  (answered by the page) and scene-domain (answered by the worker).

## Data flow: a frame and an edit

**Startup.** The viewport transfers its canvas to the engine worker and sends
`Init`. The worker builds the renderer, runs the offscreen frame loop, and posts
`Ready` carrying the command manifest, the command json schema, and the standard
library. The page stores these, seeds the language worker, and syncs the plugin
set to the worker.

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

- `shapes` — `cube`, `sphere`, `glowing`, `grid`, `ring`.
- `color` — `hsv`, `gray`, `mix_color`, and named colors.
- `motion` — `spin`, `bob`, `orbit`.
- `events` — `hits`, `sensor_hits`, `other`.
- `input` — `axis_x`, `axis_z`, `held`.
- `random` — `random_color`, `random_point`, `random_pick`.

The library is sent to the page on `Ready` so the editor shows its source and the
language service offers its helpers.

## The editor and language service

The code editor (`components/editor_pane.rs`) is a native textarea for editing
with a Rust highlight `<pre>` layer behind it sharing the same box. Highlighting
is a hand-written rhai scanner (`highlight.rs`); command tokens are colored from
the manifest, so the color set never drifts from what a script can call.

The language service is reflective. The worker derives the command vocabulary
from `command_manifest` and `command_schema`, and the highlighter, the reference
overlay, and the language worker all read from it. Add a free function to
`nightshade-api` and it becomes a `Command`, then it lights up across the editor
with no editor changes.

## Theming

`data-theme` on the document root selects a theme; CSS variable blocks under
`[data-theme="..."]` in `public/styles.css` define each. The id persists in local
storage (`theme.rs`). The toolbar picker sets `state.theme`; an effect applies
and persists it.

## Claude / MCP integration (milestone 2)

The toolbar Claude toggle opens the chat pane (`components/chat.rs`), which asks
the desktop shell over webview IPC to start the bridge and connects to the chat
relay websocket, rendering the Claude subprocess's stream-json. The agent surface
(`AgentRequest`/`AgentResponse`) spans the editor domain (buffers, panels,
plugins, answered by the page) and the scene domain (entities, screenshot,
answered by the worker). The desktop-hosted MCP server that exposes these as
tools and pipes the `claude` subprocess is the remaining piece of this milestone.

## The editor-manipulation API and vim

A second plugin kind, editor plugins, runs in a page-side rhai engine
(`src/editor_plugins.rs`). An editor plugin's `on_key()` reads `key`, `mode`,
`ctrl`/`shift`/`alt`, and a persistent `state` map, and pushes ops the host
applies to the code buffer: `Consume`, `SetMode`, `Insert`, `Move`, `MoveLine`,
`LineStart`/`LineEnd`, `NextWord`/`PrevWord`, `DeleteForward`/`DeleteBackward`,
`DeleteLine`, `SetStatus`. The dispatch mirrors the scene-plugin model, on the
editor instead of the scene, and runs synchronously in the keydown handler so
modal editing has no latency.

The vim keybindings layer ships as an editor plugin built on this, alongside an
editor-plugin template. Both are editable and toggleable in the plugin panel, so
the bindings are tuned by editing rhai, live. nightshade's `Command`/`Event` bus
is closed, so this is a neon layer on top, sharing the rhai authoring experience.

The buffer and cursor surface is implemented. Multi-window, panel, tile, and
terminal manipulation extend the same op vocabulary once that UI infrastructure
(a tiling layout, multiple buffers, integrated terminals) exists, which is the
next build.

## Build

```sh
just run       # native webview over the bundle
just run-web   # serve in the browser
```

`just workers` builds the engine and language wasm into `runtime/` via
wasm-bindgen and wasm-opt; `just build` adds the trunk bundle. Path dependencies
point at a sibling `../nightshade` checkout for the live facade.
