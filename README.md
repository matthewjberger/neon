# Neon

A plugin-first 3D editor written in Rust. The UI is [Leptos](https://leptos.dev), the [nightshade](https://crates.io/crates/nightshade) engine runs in a web worker on an `OffscreenCanvas`, and you write plugins in [rhai](https://rhai.rs). There is no npm anywhere in the stack, not even transitively. Highlighting, completion, and diagnostics are all Rust.

Plugins come in two kinds, both authored in the editor and both with their source on view:

- **Scene plugins** run in the engine worker. They push `Command`s and read `Event`s each tick, building on a standard rhai library of procedural helpers.
- **Editor plugins** run on the page and drive the editor itself through keystrokes. The default set is a Spacemacs layer: vim modal editing plus an `SPC` leader with a which-key menu. A Vim layer ships alongside it.

The standard library and the built-in editor plugins are visible and editable, but locked from writing, so you can read exactly how everything works.

## Run

```sh
just run       # native webview window over the web bundle
just run-web   # serve in the browser through trunk at http://127.0.0.1:8080
just test      # run the language-worker tests
```

`just init` installs the pinned toolchain (rust, wasm-bindgen, wasm-opt, trunk) through mise. Rendering is WebGPU, so use a browser or platform webview with WebGPU and OffscreenCanvas-in-workers support (Chromium 113+, Firefox 141+).

## Editing

The editor is modal by default through the Spacemacs plugin.

- `i` `a` `A` `o` enter insert, `Esc` leaves it.
- `h` `j` `k` `l` move, `0` `$` jump to the line ends, `w` `b` move by word.
- `x` deletes a character, `dd` deletes a line.
- `SPC` opens the leader menu. The bottom panel lists the next keys and what they do, narrowing as you go: `SPC w` for windows, `SPC t` for toggles, `SPC b` for buffers, `SPC p` for plugins.
- `Ctrl+Shift+P` or `:` opens the command palette. `F1` or `SPC ?` opens the keybinding help.

Every editor action is a named command in one registry, so the palette, the leader menus, and the plugins all drive the same set.

## Architecture

Three isolated contexts, one wire format. See `DESIGN.md` for the full design.

- **Main thread (`src/`, the `neon` crate):** the Leptos UI. The code editor (a Rust syntax-highlighting overlay over a native textarea), the command palette, the which-key menu, the plugin manager, the console, the reference, and the engine viewport host. No engine, no rhai, no npm.
- **Engine worker (`worker/`):** the `nightshade-api` facade plus the offscreen renderer. Runs the scene plugins each tick with `run_scripts`, applies the `Command`s they produce, renders the scene, and exports the command manifest and the standard library to the page.
- **Language worker (`lang/`):** links only `rhai`. Compile-checks plugin source and validates command calls against the manifest, off the render thread.
- **Desktop (`desktop/`):** a `wry` webview shell that serves and embeds the web bundle, plus the Claude bridge.

Supporting crates and folders: `protocol/` holds the shared serde wire types, `worker/stdlib/` is the standard rhai library for scene plugins, and `editor_stdlib/` is the built-in editor plugins (Spacemacs, Vim, a template).

## Plugins

A scene plugin is a rhai script with `on_start` and `on_tick`. Inside them you push `Command`s to the `commands` array and read this frame's `Event`s from `events`. The standard library adds higher-level builders, called as methods on `commands` or `events`:

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

An editor plugin is a rhai script with `on_key`. It reads `key`, `mode`, `ctrl`, `shift`, `alt`, and a persistent `state` map, then pushes ops to `ops`: move the caret, change mode, run a command, open the palette, or publish a which-key menu. The Spacemacs plugin in `editor_stdlib/` is the working reference.

## Claude

The toolbar has a Claude toggle that opens a chat pane. With the desktop shell running, Claude reaches the editor through an MCP bridge: it can read what the editor sees, get and set buffer text, and run editor commands. Ask it to drive the editor and it will.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
