# Your First Plugin

Two short walk-throughs, one per kind.

## A scene plugin

Open the Installed view and click New Plugin. You get a starter:

```rhai
fn on_start() {
    commands.cube([0.0, 0.5, 0.0], RED);
}

fn on_tick() {
}
```

Make it rain spheres. Edit `on_tick` to draw a few each frame:

```rhai
fn on_tick() {
    if random() < 0.3 {
        commands.draw_sphere(random_point(5.0), 0.2, random_color());
    }
}
```

Save and the viewport updates. `draw_sphere` is immediate, so it redraws every
frame with no entity to track. `random`, `random_point`, and `random_color` come
from the standard library. If you mistype a command, the language worker flags it
in the diagnostics strip.

## An editor plugin

Install the editor template from the plugin manager (Starters), or create a new
plugin and paste an `on_key`. A plugin that wraps the line in a banner on a
chord:

```rhai
fn on_key() {
    if ctrl && key == "b" {
        ops.push("Consume");
        ops.push("LineStart");
        ops.push(#{ Insert: "// === " });
        ops.push("LineEnd");
        ops.push(#{ Insert: " ===" });
    }
}
```

`Consume` stops the key from typing normally. The rest are ops the host applies
to the buffer (see the [Op Vocabulary](op-vocabulary.md)). Because every text op
records undo, `Ctrl+Z` reverts the whole banner in one step.

## Add a leader entry

To hang an action off the leader, edit `spacemacs.rhai` from the plugin manager.
Add an item to a menu builder and a branch in `on_key` that pushes a
`RunCommand` or an op. Save, and the binding is live. See
[The Spacemacs Leader](spacemacs-leader.md).
