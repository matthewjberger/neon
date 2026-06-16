# What Neon Is

Neon is a general-purpose code editor that happens to embed a 3D engine. It edits
files from disk, speaks to rust-analyzer, and is driven by a modal keymap, like
any editor. The engine is there because the same rhai plugin model that extends
the editor also scripts a live scene.

## Principles

- **All Rust, no npm.** Every dependency is a Rust crate. The only JavaScript is
  the per-worker bootstrap, a few lines each, with no packages. This rules out
  CodeMirror and tree-sitter (npm and C respectively). Highlighting is a
  hand-written multi-language scanner in Rust. Richer language intelligence comes
  from LSP, not a bundled grammar.
- **Data-oriented, not OOP.** Page state is a `Copy` struct of signals, behavior
  is free functions, components are plain functions. Nothing is an object that
  owns the app, the engine, or the workers.
- **One wire format.** Every cross-context message is serde, defined once in the
  `protocol` crate.
- **Plugins easy and plentiful.** A plugin is a small rhai file. The standard
  library does the heavy lifting so a useful plugin is a few lines, and the
  source for every plugin and the standard library is visible in the app.

## Two plugin kinds, one experience

Neon has editor plugins and scene plugins. Editor plugins extend the editor:
keybindings, modal editing, and commands. Scene plugins drive the 3D view:
spawn, animate, and react to events. Both are rhai, both have hooks, and both
show their source in the app, so it stays one experience.

The nightshade `Command` and `Event` bus is closed (engine-defined, no
custom-emit), so the editor API is a neon layer on top, not a ride on that bus.

## Where the work happens

Neon splits across four isolated contexts so nothing blocks the editor. The next
chapter, [The Four Contexts](contexts.md), lays them out.
