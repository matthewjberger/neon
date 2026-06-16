# Editor Op Reference

Every op an editor plugin can push to `ops`. A plain string has no payload, a
one-key map carries one. The host applies them in `editor_plugins.rs`. For the
how-to, see [Editor Plugins](editor-plugins.md).

## Control

| Op | Payload | Effect |
| --- | --- | --- |
| `Consume` | none | stop the key from typing normally |
| `SetMode` | string | set the mode label |
| `SetStatus` | string | show a transient status message |
| `OpenPalette` | none | open the command palette |
| `RunCommand` | string | run a command by id |
| `ShowMenu` | map | publish a which-key menu |
| `HideMenu` | none | clear the which-key menu |

## Caret motion

| Op | Payload | Effect |
| --- | --- | --- |
| `Move` | int | move by characters (negative reverses) |
| `MoveLine` | int | move by lines |
| `LineStart` `LineEnd` | none | to line start or end |
| `SmartLineStart` | none | toggle first non-blank and line start |
| `NextWord` `PrevWord` | none | by word |
| `FindChar` | string | to the next occurrence on the line |

## Edits

| Op | Payload | Effect |
| --- | --- | --- |
| `Insert` | string | insert at the caret |
| `DeleteForward` `DeleteBackward` | int | delete characters |
| `DeleteLine` | none | delete the line |
| `DeleteToLineEnd` | none | delete to line end |
| `DeleteWordForward` `DeleteWordBackward` | none | delete a word |
| `DuplicateLine` | none | duplicate the line below |
| `MoveLineUp` `MoveLineDown` | none | shuffle the line |
| `JoinLines` | none | pull the next line up |
| `Indent` `Outdent` | none | by four spaces |
| `ToggleComment` | string | comment or uncomment with the marker |

## Transforms

| Op | Payload | Effect |
| --- | --- | --- |
| `UpperCaseWord` `LowerCaseWord` | none | the word under the caret |
| `SortLines` | none | sort all lines |
| `DeleteTrailingWhitespace` | none | trim every line |

## A menu value

`ShowMenu` takes a map of `title` and `items`, where each item is a map of `key`
and `label`:

```rhai
#{ ShowMenu: #{ title: "+Menu", items: [
    #{ key: "a", label: "Action" },
] } }
```
