# Introduction

Neon is a code editor written in Rust. The UI is [Leptos](https://leptos.dev),
it edits files from disk with rust-analyzer for Rust projects, and it is
extensible through [rhai](https://rhai.rs) plugins. Two plugin kinds share one
authoring experience: editor plugins add editor functionality (keybindings,
modal editing, commands), and scene plugins drive a live 3D view rendered by the
[nightshade](https://github.com/matthewjberger/nightshade) engine in a web
worker.

The whole stack is Rust plus a few lines of wasm bootstrap JavaScript. No npm,
no bundler, no JavaScript framework.

## What you get

- A modal editor with a deep Spacemacs leader, modeled on VSpaceCode, and a Vim
  layer. The keymap is a rhai plugin you can read and edit live.
- Full rust-analyzer support: diagnostics, completion, hover, signature help, go
  to definition, references, symbols, rename, code actions, formatting, and
  diagnostic stepping.
- Files from disk: open a folder, browse the tree, search the project, and the
  session restores on launch.
- A live 3D viewport you script in rhai, with a reflective command reference and
  in-editor diagnostics.
- Claude in the editor through MCP, so an agent can read state, edit buffers, and
  drive the scene.

## How to read this book

[What Neon Is](what-neon-is.md) and [The Four Contexts](contexts.md) explain the
shape of the system. The Editor section covers the day-to-day surface. Plugins
covers both plugin kinds and the standard library. Language Support covers
rust-analyzer. The Reference section has the full keybinding, command, and op
tables.

If you just want to run it, start with [Installation](installation.md) and
[Running Neon](running.md).
