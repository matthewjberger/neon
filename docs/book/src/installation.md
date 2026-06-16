# Installation

Neon builds from source. The toolchain is pinned with
[mise](https://mise.jdx.dev), and tasks run through
[just](https://github.com/casey/just).

## Prerequisites

- A recent Rust toolchain (the repo pins one through mise).
- `just` and `mise` on your path.
- A platform with WebGPU and OffscreenCanvas-in-workers support, since the engine
  renders through WebGPU in a worker. The bundled desktop shell uses the system
  webview, so a current Chromium or WebKit is enough.
- For rust-analyzer support, `rustup` and `cargo`. Neon discovers rust-analyzer
  through `rustup`, the same way VS Code does, so no separate install is needed.

## Get the tools

```sh
just init
```

This installs the pinned tools through mise: the Rust toolchain, `wasm-bindgen`,
`wasm-opt`, and `trunk`.

## Clone

Neon depends on a sibling checkout of nightshade for the engine facade. Clone
both next to each other:

```sh
git clone https://github.com/matthewjberger/neon
git clone https://github.com/matthewjberger/nightshade
```

The path dependencies in `Cargo.toml` point at `../nightshade`, so the layout
matters.

## Next

Once the tools are in place, see [Running Neon](running.md).
