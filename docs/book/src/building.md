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

## The book

The book lives in `docs/book` and builds with mdBook:

```sh
cd docs/book
just serve     # or: mdbook serve --open
```

It deploys to GitHub Pages on every push to `main` through
`.github/workflows/pages.yml`. The workflow builds the book with mdBook and
publishes it, and Pages serves it at the project URL.

## Publishing crates

The workspace carries the package metadata cargo needs to publish. The binary and
internal crates set `publish = false`, and the publishable crates declare their
description, repository, license, and keywords. Run `cargo publish` on a crate
when you intend to release it.
