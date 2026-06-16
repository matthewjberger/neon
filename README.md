# Neon

A plugin-first 3D editor. The UI is [Leptos](https://leptos.dev) in Rust, the [nightshade](https://crates.io/crates/nightshade) engine runs in a web worker on an `OffscreenCanvas`, and you write plugins in [rhai](https://rhai.rs) that produce `Command` and consume `Event`. Plugins are authored in the editor itself, against a standard rhai library of procedural helpers, and every plugin's source (and the standard library's) is visible and editable in the app.

The whole stack is Rust plus a few lines of wasm bootstrap JavaScript. No npm, no bundler, no JavaScript framework. Highlighting, completion, and diagnostics are Rust.

## Run

```sh
just run       # native webview window over the web bundle
just run-web   # serve in the browser via trunk
```

`just init` installs the pinned toolchain (rust, wasm-bindgen, wasm-opt, trunk) through mise. Rendering is WebGPU, so use a browser or platform webview with WebGPU and OffscreenCanvas-in-workers support (Chromium 113+, Firefox 141+).

## Architecture

Three isolated contexts, one wire format. See `DESIGN.md` for the full design.

- **Main thread (`src/`, the `neon` crate):** the Leptos UI. The code editor (a Rust syntax-highlighting overlay over a native textarea), the plugin manager, the command console, the reference, and the engine viewport host. No engine, no rhai, no npm.
- **Engine worker (`worker/`):** the `nightshade-api` facade plus the offscreen renderer. Runs the plugins each tick with `run_scripts`, applies the `Command`s they produce, renders the scene, and exports the command manifest and the standard library to the page.
- **Language worker (`lang/`):** links only `rhai`. Compile-checks plugin source and validates command calls against the manifest, off the render thread.
- **`protocol/`:** the shared serde wire types for both workers.
- **`worker/stdlib/`:** the standard rhai library, the procedural helpers plugins build on.
- **`desktop/`:** a `wry` webview shell that serves and embeds the web bundle.

## Plugins

A plugin is a rhai script with `on_start` and `on_tick` hooks. Inside them you push `Command`s to the `commands` array and read this frame's `Event`s from `events`. The standard library adds higher-level builders on top, called as methods on `commands` or `events`:

```rhai
fn on_start() {
    commands.grid(8, 8, 1.5, hsv(0.6, 0.7, 1.0));
}

fn on_tick() {
    for hit in events.hits(self) {
        commands.push(#{ Despawn: #{ entity: other(hit, self) } });
    }
}
```

## License

Dual-licensed under MIT or Apache-2.0, at your option.
