# Command Reference

Every command in the registry (`commands.rs`). Each has an id an editor plugin
can invoke with `#{ RunCommand: "id" }`, and most appear in the palette. This is
the single set the palette, the leader, the menus, and plugins all drive.

## Panes and tabs

`split-right`, `split-below`, `close-split`, `focus-other`, `focus-next`,
`focus-prev`, `balance-splits`, `close-tab`, `next-tab`, `prev-tab`.

## Views and panels

`show-installed`, `show-manager`, `show-files`, `show-search`,
`toggle-preview`, `toggle-console`, `toggle-reference`, `toggle-control-panel`,
`toggle-chat`, `toggle-lsp-log`.

## Files

`open-folder`, `save-file`, `save-all`.

## Find and jump

`find`, `jump-word`, `jump-line`, `jump-char`.

## Language features

`go-to-definition`, `go-to-type-definition`, `go-to-implementation`,
`find-references`, `jump-symbol`, `workspace-symbols`, `hover`, `signature-help`,
`rename-symbol`, `code-action`, `format-document`, `next-error`, `prev-error`.

## Scene and runtime

`run-pause`, `reset-scene`.

## Editor

`new-plugin`, `open-palette`, `open-help`, `next-theme`.

## Generated entries

The palette also lists a `Theme: ...` entry per installed theme and an `Open: ...`
entry per installed plugin and built-in module. Those are generated from the
current sets, not fixed ids.
