# The Four Contexts

Neon runs in four isolated execution contexts, each doing what it is best at.
None can block the others.

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
        render, agent     validate              chat relay, fs, lsp)
```

## Main thread (the `neon` crate)

The Leptos UI. The code editor, plugin manager, console, reference, viewport
host, toolbar, and chat. Data-oriented: state is a `Copy` struct of signals
(`state.rs`), behavior is free functions, components are plain `#[component]`
functions. No engine, no rhai, no npm.

## Engine worker (`worker/`)

The `nightshade-api` facade plus the offscreen renderer. It owns the transferred
`OffscreenCanvas`, runs the scene plugins through `run_scripts` each tick,
applies the `Command`s they produce, renders, and answers scene-domain agent
requests. The only place that touches the engine.

## Language worker (`lang/`)

Links only `rhai`. It compile-checks plugin source and flags unknown command
calls, off the render thread, so editing a scene plugin never stalls rendering.

## Desktop shell (`desktop/`)

A `wry` webview that serves and embeds the web bundle, and hosts four relays the
page cannot run itself:

- The Claude MCP bridge and chat relay.
- A filesystem bridge: disk access through `rfd` and `tokio::fs`, plus project
  search through the `ignore` crate.
- A language-server bridge: it discovers rust-analyzer through `rustup`, spawns
  it, and frames LSP JSON-RPC over stdio.

The only JavaScript is `runtime/worker.js` and `runtime/lang_worker.js`, a few
lines each that boot the respective wasm modules and buffer early messages. No
packages.
