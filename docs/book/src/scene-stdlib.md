# The Scene Standard Library

The standard library is rhai embedded into the worker and prepended to every
scene plugin, so its helpers are always in scope. It does the heavy lifting, so a
useful plugin is a few lines. The source lives in `worker/stdlib/` and is
viewable in the editor as read-only built-in modules.

## Modules

- **shapes**: `commands.cube`, `commands.sphere`, `commands.glowing`,
  `commands.grid`, `commands.ring`.
- **color**: `hsv`, `gray`, `mix_color`, and the named colors `RED`, `GREEN`,
  `BLUE`, `WHITE`.
- **motion**: `commands.spin`, `commands.bob`, `commands.orbit`. These take `dt`
  or `time` explicitly, since rhai functions do not capture outer scope.
- **events**: `events.hits`, `events.sensor_hits`, `events.other`.
- **input**: `axis_x`, `axis_z`, `held`.
- **random**: `random_color`, `random_point`, `random_pick`.

## Why helpers take dt and time

A rhai function does not capture the scope it was defined in. A motion helper
cannot reach `dt` or `time` on its own, so you pass them in:

```rhai
fn on_tick() {
    commands.spin(named("cube"), dt * 2.0);
    commands.bob(named("ball"), time, 0.5);
}
```

## Calling style

Builders are methods on `commands`, filters are methods on `events`, and the rest
are free functions:

```rhai
fn on_tick() {
    let c = hsv(time * 0.1, 0.7, 1.0);
    for index in 0..8 {
        commands.draw_sphere(random_point(5.0), 0.2, random_color());
    }
}
```

## Where it comes from

The library is sent to the page on `Ready`, so the editor shows its source and
the language service offers its helpers in completion and the reference. Add a
helper to a stdlib module and it shows up across the editor with no other change.
