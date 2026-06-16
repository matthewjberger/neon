# Scene Plugins

A scene plugin scripts the 3D view. It has two optional hooks and a handful of
values always in scope.

```rhai
fn on_start() {
    commands.cube([0.0, 0.5, 0.0], RED);
}

fn on_tick() {
}
```

- `on_start()` runs once when the plugin loads.
- `on_tick()` runs every frame.

## What is in scope

Inside a scene plugin these are always available:

| Name | What it is |
| --- | --- |
| `commands` | the array you push api `Command`s to |
| `events` | this frame's `Event`s |
| `dt` | seconds since the last frame |
| `time` | seconds since start |
| `keys` | the keyboard state |
| `mouse` | the pointer state |
| `named` | look up an entity by name |
| `tagged` | look up entities by tag |
| `replies` | results returned by commands that answer |

## Commands

You build a scene by pushing commands. The raw form is a `Command` value, but the
standard library adds higher-level builders called as methods on `commands`, so
most plugins never write a raw command:

```rhai
fn on_start() {
    commands.cube([0.0, 0.5, 0.0], hsv(0.6, 0.7, 1.0));
    commands.ring(12, 3.0, [1.0, 0.5, 0.2, 1.0]);
}
```

Commands a plugin produces apply as one deferred batch at the end of the tick.

## Immediate drawing

Some helpers draw for a single frame instead of spawning a tracked entity. Redraw
them every tick and they animate with no entity bookkeeping:

```rhai
fn on_tick() {
    let radius = 4.0 + sin(time * 2.0) * 0.7;
    commands.draw_sphere([radius, 1.0, 0.0], 0.3, WHITE);
}
```

## Events

Read this frame's events from `events`, with standard-library filters:

```rhai
fn on_tick() {
    for hit in events.hits() {
        // react to a collision
    }
}
```

## Editing live

Typing in a scene plugin updates its source. After a short debounce the page
sends the whole plugin set to the worker, which replays the plugins from a clean
stage, so editing or toggling never leaves a prior run's entities behind. In
parallel the source goes to the language worker, which compile-checks it and
flags unknown command calls. Diagnostics show in a strip under the editor.

The reflective reference (the Reference view) lists every command and standard
library helper, derived from the live manifest, so the vocabulary is always in
front of you. The next chapter covers the
[standard library](scene-stdlib.md).
