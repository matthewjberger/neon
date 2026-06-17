# Building and Shipping

Tasks run through `just`. The recipes wrap the wasm toolchain so you do not call
`trunk` or `wasm-bindgen` by hand.

## Common recipes

```sh
just run        # native webview window over the bundle (the full editor)
just run-web    # serve in the browser at http://127.0.0.1:8080
just workers    # build the engine and language wasm into runtime/
just build      # workers plus the trunk bundle
just lint       # clippy across the wasm crates and the desktop crate
just init       # install the pinned tools through mise
```

## What a build is made of

- `just workers` compiles the `worker` and `lang` crates to
  `wasm32-unknown-unknown`, runs `wasm-bindgen` to generate the bindings into
  `runtime/`, and `wasm-opt` to optimize them.
- `trunk build` bundles the Leptos page.
- `cargo run -p desktop` launches the webview shell over the bundle.

Path dependencies point at a sibling `../nightshade` checkout for the live
facade, so keep the two repos next to each other.

## The Rust dev loop

Neon runs a real terminal inside the editor. The desktop shell opens a
pseudo-terminal on the platform shell, parses its output into a screen grid, and
renders it live so full-screen programs, colors, and prompts all work. Toggle it
with `SPC '` and type into it like any terminal.

The `SPC c` menu runs cargo straight into that terminal:

- `SPC c c` check, `SPC c b` build, `SPC c t` test, `SPC c r` run.
- `SPC c k` sends an interrupt, `SPC c o` toggles the terminal.

rust-analyzer already surfaces check-on-save diagnostics; this is for running
tests and the binary and watching the output live. The same commands are in the
palette.

## The book

The book lives in `docs/book` and builds with mdBook:

```sh
cd docs/book
just serve     # or: mdbook serve --open
```

## Deploying to Pages

`.github/workflows/pages.yml` publishes to GitHub Pages on every push to `main`.
It checks out neon and a sibling nightshade, builds the workers and a release web
bundle, builds the book with mdBook, and lays them out as one site: the live
editor at the project root and the book under `book`. So the editor runs at
`/neon/` and the book reads at `/neon/book/`.

## Publishing crates

The workspace carries the package metadata cargo needs to publish. The binary and
internal crates set `publish = false`, and the publishable crates declare their
description, repository, license, and keywords. Run `cargo publish` on a crate
when you intend to release it.
