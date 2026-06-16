# The Control Panel

The control panel is a master surface for dispatching any command and watching
the API log. Toggle it from the View menu, the palette, or `SPC t o`.

The engine core only accepts commands and emits events, so this panel exercises
that surface and shows every call as it happens.

## What it has

- **Editor commands.** A button for every entry in the command registry, so you
  can fire any editor action by hand.
- **Scene API.** Spawn cube, reset scene, and run or pause, plus a button per
  command in the live manifest. Each manifest button submits a templated JSON
  command with type-defaulted arguments, so you can exercise the whole scene API
  and see the result.
- **API log.** The unified log of every command and event. Scene command and
  event traffic streams in from the worker, and every editor-command dispatch is
  logged too, so the panel records all traffic. A Clear button empties it.

## The unified log

There is one log (`state.log`). The worker's per-tick `Report` feeds it the
scene command and event traffic, and `state.log_api` adds editor dispatches and
scene submissions. The console and the control panel both read it, so what you
see in the console is the same stream the control panel filters and exercises.
