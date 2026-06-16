# Keybinding Reference

The default keymap is the Spacemacs editor plugin. The bindings are rhai in
`editor_stdlib/spacemacs.rhai`, so this table reflects the shipped default and
you can change any of it live. The in-app help (`SPC ?` or F1) lists the same
set.

## Global

| Keys | Action |
| --- | --- |
| `Ctrl+Shift+P` | command palette |
| `F1` | help and keybindings |
| `Ctrl+F` | find in buffer |
| `Ctrl+Z` / `Ctrl+Y` | undo / redo |
| `Tab` | indent |

## Normal mode

| Keys | Action |
| --- | --- |
| `i` `a` `A` `I` | insert before, after, line end, first non-blank |
| `o` | open a line below and insert |
| `Esc` | back to normal mode |
| `h` `j` `k` `l` | move left, down, up, right |
| `0` `$` | line start, line end |
| `w` `b` | word forward, back |
| `/` | find in buffer |
| `x` `X` | delete character forward, back |
| `D` `C` | delete, change to line end |
| `J` | join the next line up |
| `dd` | delete line |
| `K` | hover |
| `gd` | go to definition |

## Leader (SPC)

| Keys | Action |
| --- | --- |
| `SPC SPC` | command palette |
| `SPC TAB` | last buffer (previous tab) |
| `SPC /` | search the project |
| `SPC ;` | toggle comment |
| `SPC ?` | help |
| `SPC a` | code action |
| `SPC r` | run or pause |
| `SPC T` | next theme |
| `SPC b` `b/d/n/p` | list / close / next / previous tab |
| `SPC f` `f/s/S/t/n` | open folder / save / save all / tree / new plugin |
| `SPC w` `v/s/d/h/l/w/=` | split right / below / close / focus prev / next / other / balance |
| `SPC s` `s/p/j/J` | find / search project / jump symbol / workspace symbols |
| `SPC j` `j/l/w/i/+` | jump char / line / word / symbol / format |
| `SPC g` `g/t/i/r/R/s/S` | definition / type / impl / references / rename / symbol / workspace |
| `SPC h` `h/s/k` | hover / signature help / keybindings |
| `SPC e` `n/p/l` | next error / previous error / rust-analyzer log |
| `SPC x` `;/d/j/k/J/>/</u/U/s/w/r/.` | comment / duplicate / move down / up / join / indent / outdent / lower / upper / sort / trim / rename / code action |
| `SPC t` `p/c/r/a/o/l/t` | preview / console / reference / Claude / control panel / LSP log / theme |
| `SPC p` `f/t` | search / file tree |
| `SPC P` `n/m/i` | new plugin / manager / installed |
