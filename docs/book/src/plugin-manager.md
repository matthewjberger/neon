# The Plugin Manager

The Extensions view is the plugin manager. It lists the bundled catalog grouped
by category, with an install or uninstall control on each entry, so the workspace
explorer only shows what you have installed.

## Categories

The catalog is grouped so related plugins sit together:

- **Keybinding layers**: Spacemacs, Vim, Emacs keys.
- **Editing**: auto pairs, better escape, line tools, word delete, join lines,
  move lines.
- **Motions**: jump to char, smart home, blank lines.
- **Comments**: comment toggle, the comment object.
- **Starters**: the scene template, the editor template.
- **Visuals**: the scene examples (color grid, confetti, orbits, fireworks, and
  more).

## Installing

Click Install on a catalog entry to add it to the relevant set (editor or scene)
and open its source. Click Uninstall to remove it. Installing a modal layer
disables any other modal layer, since only one can own the keyboard in normal
mode.

## The installed set

The Installed view lists what you have, with enable toggles and a New Plugin
button. Scene plugins sync to the worker on every change. The standard library
shows here too, read-only, so every helper a plugin builds on is one click away.

## Defaults

A fresh install starts with the scene template loaded and the Spacemacs editor
plugin enabled. Everything else is in the catalog to install.
