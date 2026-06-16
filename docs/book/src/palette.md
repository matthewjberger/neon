# The Command Palette

The palette is a filterable, keyboard-driven list of every editor command. Open
it with the command center in the top bar, `Ctrl+Shift+P`, `SPC SPC`, or `:`.

## How it works

Type to filter, arrow keys to move, Enter to run, Escape to close. The list comes
from one registry (`commands.rs`): the static operations, a theme per installed
theme, and an open command per installed plugin and built-in module.

## One registry, many drivers

The palette, the Spacemacs leader menus, the right-click menus, and editor
plugins all drive the same command set. An editor plugin invokes any command by
id through a `RunCommand` op, so plugins dictate what the editor does, not just
what the buffer holds.

The commands cover panes and tabs, files, the sidebar views, find, the jumps,
the language features (definition, references, symbols, hover, signature help,
rename, code actions, format, diagnostic stepping), run or pause, themes, the
control panel, and help. The full list is in the
[Command Reference](reference-commands.md).
