# Modal Editing

The default keymap is a Spacemacs layer: Vim modal editing plus an `SPC` leader.
A Vim layer ships alongside it. Both are editor plugins, so the bindings are rhai
you can read and edit live (see [Editor Plugins](editor-plugins.md)).

## Modes

The editor has a mode label, shown in the top bar. In normal mode keys are
commands. In insert mode keys type. `Escape` returns to normal mode and steps the
caret back one, the Vim way.

## Normal-mode keys

| Key | Action |
| --- | --- |
| `i` `a` `A` `I` | insert before, after, at line end, at first non-blank |
| `o` | open a line below and insert |
| `h` `j` `k` `l` | move left, down, up, right |
| `0` `$` | line start, line end |
| `w` `b` | word forward, word back |
| `x` `X` | delete character forward, back |
| `D` `C` | delete, change to line end |
| `dd` | delete line |
| `J` | join the next line up |
| `/` | find in buffer |
| `K` | hover (describe the symbol) |
| `gd` | go to definition |
| `SPC` | open the leader menu |
| `:` | command palette |

## Only one modal layer at a time

Vim and Spacemacs each claim the whole keyboard in normal mode, so neon treats
them as exclusive. Enabling one disables the other
(`plugins::enforce_modal_exclusivity`). The non-modal layers (Emacs keys, auto
pairs, and so on) stack freely on top.

The leader is the heart of the keymap. The next chapter,
[The Spacemacs Leader](spacemacs-leader.md), walks the whole tree.
