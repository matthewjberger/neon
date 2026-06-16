# The Plugin Model

A plugin is a small rhai file with hooks. Neon has two kinds, and they share
authoring, so writing one feels like writing the other.

- **Scene plugins** drive the 3D view. They run in the engine worker and push
  api `Command`s.
- **Editor plugins** extend the editor. They run on the page and push ops the
  host applies to the buffer and the editor.

Both are a `PluginSource`: an id, a name, rhai source, and an enabled flag. The
page owns the installed sets, persists them to local storage, and is the single
authority for syncing scene plugins to the worker.

## Where each kind runs

Scene plugins run in the worker so they can touch the engine without blocking the
page. The worker rebuilds the engine's global scripts from the enabled scene
plugins, prepends the standard library so its helpers are in scope, and runs
everything each tick through `run_scripts`.

Editor plugins run in a neon-owned rhai engine on the page, synchronously in the
keydown handler, so modal editing has no latency.

## Two sets, one catalog

The plugin manager (the Extensions view) lists a bundled catalog grouped by
category: keybinding layers, editing tools, motions, comments, starters, and
visuals. Install a catalog entry into the relevant set, or create a fresh plugin.
Every plugin's source and the standard library source are viewable in the editor,
so nothing is hidden.

## The closed bus

The nightshade `Command` and `Event` bus is closed: it is engine-defined, with no
custom-emit. So the editor API is a neon layer on top of the page, not a ride on
that bus. Scene plugins speak the engine's vocabulary, editor plugins speak
neon's op vocabulary, and the two never cross.

The next chapters cover each kind in depth: [Scene Plugins](scene-plugins.md) and
[Editor Plugins](editor-plugins.md).
