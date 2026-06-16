# The Editor Op Vocabulary

An editor plugin pushes ops to `ops`. The host applies them to the buffer, the
caret, and the editor (`editor_plugins.rs`). This chapter is the working
reference. The full table is in the [Editor Op Reference](reference-ops.md).

## Form

Two shapes:

- A plain string for an op with no payload: `ops.push("DuplicateLine")`.
- A one-key map for an op with one payload: `ops.push(#{ Move: 1 })`.

## Control

| Op | Effect |
| --- | --- |
| `"Consume"` | stop the key from typing normally |
| `#{ SetMode: "..." }` | set the mode label |
| `#{ SetStatus: "..." }` | show a transient status message |
| `"OpenPalette"` | open the command palette |
| `#{ RunCommand: "id" }` | run any command in the registry |
| `#{ ShowMenu: menu }` | publish a which-key menu |
| `"HideMenu"` | clear the which-key menu |

## Caret motion

`#{ Move: n }`, `#{ MoveLine: n }`, `"LineStart"`, `"LineEnd"`,
`"SmartLineStart"`, `"NextWord"`, `"PrevWord"`, `#{ FindChar: "x" }`.

## Edits

`#{ Insert: "text" }`, `#{ DeleteForward: n }`, `#{ DeleteBackward: n }`,
`"DeleteLine"`, `"DeleteToLineEnd"`, `"DeleteWordForward"`,
`"DeleteWordBackward"`, `"DuplicateLine"`, `"MoveLineUp"`, `"MoveLineDown"`,
`"JoinLines"`, `"Indent"`, `"Outdent"`, `#{ ToggleComment: "//" }`.

## Transforms

`"UpperCaseWord"`, `"LowerCaseWord"`, `"SortLines"`,
`"DeleteTrailingWhitespace"`.

## Notes

- An op that changes text records undo through `set_buffer_text`, so plugin edits
  undo like any other.
- `RunCommand` is the bridge to everything that is not a buffer edit: panes,
  files, the language features, themes, and the panels. Anything in the
  [Command Reference](reference-commands.md) is reachable by id.
- The case transforms act on the word under the caret. `SortLines` and
  `DeleteTrailingWhitespace` act on the whole buffer.
