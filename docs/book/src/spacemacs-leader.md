# The Spacemacs Leader

Press `SPC` in normal mode and a which-key menu opens, listing the next keys and
what they do. Each prefix narrows it. The key letters follow
[VSpaceCode](https://github.com/VSpaceCode/VSpaceCode) so the muscle memory
carries over.

The keymap lives in `editor_stdlib/spacemacs.rhai`. Every leaf is bound to a real
editor command or a buffer op, so the menu is the source of truth and you can
edit it live.

## Top level

| Key | Menu or action |
| --- | --- |
| `SPC` | command palette |
| `TAB` | last buffer (previous tab) |
| `/` | search the project |
| `;` | toggle comment on the line |
| `?` | help and keybindings |
| `a` | code action |
| `r` | run or pause the scene |
| `T` | next theme |
| `b` | +Buffers |
| `f` | +Files |
| `w` | +Windows |
| `s` | +Search |
| `j` | +Jump |
| `g` | +Goto |
| `h` | +Help |
| `e` | +Errors |
| `x` | +Text |
| `t` | +Toggles |
| `p` | +Project |
| `P` | +Plugins |

## The submenus

- **`SPC b` Buffers.** `b` list, `d` close tab, `n` next, `p` previous.
- **`SPC f` Files.** `f` open folder, `s` save, `S` save all, `t` file tree,
  `n` new plugin.
- **`SPC w` Windows.** `v` or `/` split right, `s` or `-` split below, `d` or `x`
  close split, `h` `k` `W` focus previous, `l` `j` focus next, `w` focus other,
  `=` balance.
- **`SPC s` Search.** `s` find in buffer, `p` or `/` project search, `j` jump to
  symbol, `J` workspace symbols.
- **`SPC j` Jump.** `j` to char, `l` to line, `w` to word, `i` to symbol,
  `+` or `=` format the buffer.
- **`SPC g` Goto.** `g` or `d` definition, `t` type definition, `i`
  implementation, `r` references, `R` rename, `s` symbol, `S` workspace symbols.
- **`SPC h` Help.** `h` hover, `s` signature help, `k` keybindings.
- **`SPC e` Errors.** `n` next error, `p` previous error, `l` rust-analyzer log.
- **`SPC x` Text.** `;` comment, `d` duplicate, `j` move line down, `k` move up,
  `J` join, `>` indent, `<` outdent, `u` lower case, `U` upper case, `s` sort
  lines, `w` delete trailing space, `r` rename, `.` code action.
- **`SPC t` Toggles.** `p` preview, `c` console, `r` reference, `a` Claude,
  `o` control panel, `l` rust-analyzer log, `t` next theme.
- **`SPC p` Project.** `f` search, `t` file tree.
- **`SPC P` Plugins.** `n` new, `m` manager, `i` installed.

## Editing the leader

Open `spacemacs.rhai` from the plugin manager. The menu builders return the
which-key data, and `on_key` dispatches each prefix. Change a binding, save, and
it takes effect live. See [Editor Plugins](editor-plugins.md) for the op and
command vocabulary the leader pushes.
