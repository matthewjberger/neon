# Running Neon

Neon runs two ways: as a native desktop window, or in the browser.

```sh
just run       # native webview window over the web bundle
just run-web   # serve in the browser at http://127.0.0.1:8080
```

## Desktop vs browser

`just run` builds the web bundle and opens it in a [wry](https://github.com/tauri-apps/wry)
webview, then starts the desktop shell. This is the full editor: the shell hosts
the bridges that bring rust-analyzer, disk access, project search, and Claude.
Use this for real work.

`just run-web` serves the same bundle in your browser. The editor, the scene
plugins, and the 3D viewport all run, which makes it handy for quick iteration on
the editor UI itself.

## What the build does

`just run` chains a few steps:

- `just workers` builds the engine worker and the language worker to wasm and
  generates the bindings into `runtime/`.
- `trunk build` bundles the Leptos page.
- `cargo run -p desktop` launches the webview shell over the bundle.

The first build is slow because it compiles nightshade. Later builds are
incremental.

## First run

On first launch neon opens with a scene plugin loaded and the Spacemacs editor
plugin enabled. Open a folder to start editing files (see
[Files and the Workspace](files.md)), or edit the scene plugin and watch the 3D
view update live.
