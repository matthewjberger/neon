<h1 align="center">neon ⚡</h1>

<p align="center">
  <a href="https://github.com/matthewjberger/neon"><img alt="github" src="https://img.shields.io/badge/github-matthewjberger/neon-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20"></a>
  <a href="https://github.com/matthewjberger/neon/blob/main/LICENSE-MIT"><img alt="license" src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue?style=for-the-badge&labelColor=555555" height="20"></a>
</p>

<p align="center"><strong>A code editor written in Rust, with rust-analyzer and a rhai plugin sandbox.</strong></p>

<p align="center">
  <code>just run</code>
</p>

neon is a desktop code editor. The UI is [Leptos](https://leptos.dev) in Rust, the whole stack is Rust plus a few lines of wasm bootstrap, and there is no npm anywhere, not even transitively. It opens folders and files from disk, edits and saves them, and for Rust projects it runs [rust-analyzer](https://rust-analyzer.github.io) for diagnostics, completion, and hover, discovered through rustup so all you need is rustup and cargo.

The editor is extensible through [rhai](https://rhai.rs) plugins. Editor plugins add editor functionality: keybindings, modal editing, commands, and menus, the way vim and emacs plugins do. The default Spacemacs layer and the Vim layer are themselves editor plugins, editable live in the app. Scene plugins are a second kind that drive a live 3D scene rendered by the [nightshade](https://crates.io/crates/nightshade) engine in a web worker. Every plugin's source and the standard library are visible and editable.

## Run

```bash
just run       # native webview window over the web bundle
just run-web   # serve in the browser through trunk
just test      # run the language-worker tests
```

`just init` installs the pinned toolchain (rust, wasm-bindgen, wasm-opt, trunk) through mise. Rendering is WebGPU, so use a browser or platform webview with WebGPU and OffscreenCanvas-in-workers support (Chromium 113+, Firefox 141+).

## Editing

| Feature | Notes |
|---------|-------|
| Files | Open a folder and file tree, edit and save (`Ctrl+S`), reopened on launch |
| Tabs | Multiple buffers per pane, split panes (`SPC w v` / `SPC w s`), draggable dividers |
| rust-analyzer | Diagnostics, completion, and hover for Rust files, behind a consent prompt |
| Highlighting | Per-language: rust, toml, json, javascript, rhai |
| Find | In-buffer find and replace (`Ctrl+F` / `Ctrl+H`), project-wide search |
| Command palette | `Ctrl+Shift+P` or `:`, one registry the keymaps and plugins share |
| Keybindings | Spacemacs by default (vim editing plus an `SPC` leader with a which-key menu), Vim alongside |
| Claude | A chat pane that drives the editor through an MCP bridge |
| Themes | Switchable, persisted |

## Architecture

The editor runs in four isolated contexts, one serde wire format between them. See `DESIGN.md` for the full design.

- **Main thread (`src/`):** the Leptos UI. The code editor, file tree, tabs, command palette, find, search, panels, and the engine viewport host. No engine, no rhai, no npm.
- **Engine worker (`worker/`):** the `nightshade-api` facade plus the offscreen renderer. Runs the scene plugins each tick and renders the scene.
- **Language worker (`lang/`):** links only rhai. Compile-checks plugin source off the render thread.
- **Desktop (`desktop/`):** a `wry` webview that serves the bundle, plus the bridges the page cannot host itself: a filesystem bridge, a language-server bridge that spawns rust-analyzer, and the Claude MCP and chat bridges.

## Plugins

An editor plugin is a rhai script with `on_key`. It reads the keystroke, the mode, and a persistent `state` map, and pushes ops that drive the editor: move the caret, change mode, run a command from the editor's command registry, or publish a which-key menu. This is the extension surface, and it is how the Spacemacs and Vim layers are built:

```rhai
fn on_key() {
    if mode == "normal" && key == "i" {
        ops.push(#{ SetMode: "insert" });
        ops.push("Consume");
    }
}
```

A scene plugin is the other kind: a rhai script with `on_start` and `on_tick` that drives the 3D scene. It pushes `Command`s to `commands` and reads this frame's `Event`s from `events`, building on a standard library of procedural helpers:

```rhai
fn on_start() {
    commands.grid(8, 8, 1.5, hsv(0.6, 0.7, 1.0));
}
```

Both kinds are authored in the app against the same rhai experience.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
