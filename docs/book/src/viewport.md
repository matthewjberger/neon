# The Engine Viewport

The right column hosts a live 3D view rendered by nightshade. Scene plugins draw
into it, and the console shows the traffic.

## How it renders

On startup the viewport transfers its canvas to the engine worker as an
`OffscreenCanvas` and sends `Init`. The worker builds the renderer, runs the
offscreen frame loop, and posts `Ready` with the command manifest, the command
JSON schema, and the standard library. Rendering happens entirely in the worker,
off the page's main thread, through WebGPU.

## Each frame

The worker's tick runs the engine schedule, then the camera controllers, then the
scene plugins. It clears the immediate-draw pools, runs every enabled plugin
through `run_scripts`, applies the commands they produced as one deferred batch,
and buffers the tick's commands, events, and errors. The render loop drains that
buffer to the page as a `Report`, which feeds the console and the API log.

## Running and resetting

- Run or pause the plugin runtime from the top bar or `SPC r`. Pausing stops the
  tick without unloading anything.
- Reset drops everything the plugins spawned and restores the base scene.

## The reflective reference

The Reference view lists every scene command and standard-library helper, derived
from the live manifest the worker sends. The highlighter colors command tokens
from the same source, and the language worker validates against it. Add a free
function to `nightshade-api` and it becomes a command that shows up everywhere,
with no editor change. See [The Wire Protocol](protocol.md).
