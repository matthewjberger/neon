// TEMPLATE editor plugin: handle keystrokes in the code editor.
//
// on_key() runs for every keystroke. In scope:
//   key                  the key name ("a", "Enter", "Escape", "ArrowLeft", ...)
//   mode                 the current editor mode label
//   ctrl, shift, alt     modifier booleans
//   ops                  push actions here
//   state                a map that persists across keystrokes
//
// Read-only view of the buffer (the host owns the text):
//   caret                #{ line, column, offset } of the cursor, all 0-based
//   line_text            the text of the line the caret sits on
//   selection            the selected text, or "" when nothing is selected
//   word                 the word under the caret
//
// Push to ops:
//   "Consume"                       stop the key from typing normally
//   #{ SetMode: "insert" }          set the mode label
//   #{ SetStatus: "..." }           show a transient status message
//   #{ Insert: "text" }             insert text at the cursor
//   #{ Move: 1 } / #{ MoveLine: 1 } move the cursor (negative reverses)
//   "LineStart" "LineEnd" "SmartLineStart" "NextWord" "PrevWord"
//   #{ DeleteForward: 1 } #{ DeleteBackward: 1 }
//   "DeleteLine" "DeleteToLineEnd" "DeleteWordForward" "DeleteWordBackward"
//   "DuplicateLine" "MoveLineUp" "MoveLineDown" "JoinLines"
//   "Indent" "Outdent"
//   "UpperCaseWord" "LowerCaseWord" "SortLines" "DeleteTrailingWhitespace"
//   #{ ToggleComment: "//" }        comment or uncomment the line (the region
//                                   when a selection is active)
//   #{ FindChar: "x" }              jump to the next x on the line
//
// Selections and text objects (the host tracks the region anchor):
//   "Anchor" "Collapse" "SwapEnds"  start / clear / flip the selection
//   "SelectLine" "SelectWord" "SelectAll"
//   #{ SelectInner: "(" } #{ SelectAround: "\"" }   text objects (w ( [ { < " ' `)
//   "DeleteSelection" #{ ReplaceSelection: "text" }
//   #{ Surround: ["(", ")"] }       wrap the selection (or the word) in a pair
//   "Indent" "Outdent" act on the selected lines when a selection is active
//
// Kill-ring and registers:
//   "Copy" "Cut" "Paste" "PastePop"  yank, delete-yank, paste, cycle the ring
//   #{ Register: "a" }               name the register the next yank/paste uses
//
// Search and find:
//   #{ Search: "needle" } #{ SearchBack: "needle" }   buffer search (wraps)
//   "SearchNext" "SearchPrev"        repeat the last search
//   #{ FindChar: "x" } #{ FindCharBack: "x" }         on-line find (f / F)
//   #{ TillChar: "x" } #{ TillCharBack: "x" }         on-line till (t / T)
//   "RepeatFind" "RepeatFindBack"    repeat the last find (; / ,)
//
// Repeat and jumps:
//   "Repeat"                         replay the last buffer change (the dot)
//   #{ Goto: 42 } "BufferStart" "BufferEnd" "Center"
//
// Drive the rest of the editor by id (anything in the command palette):
//   #{ RunCommand: "format-document" }    "go-to-definition" "find-references"
//   #{ RunCommand: "rename-symbol" }      "code-action" "hover" "jump-symbol"
//   #{ RunCommand: "next-error" }         "split-right" "next-tab" "find" ...
//   "OpenPalette"
//
// Build an SPC-style which-key menu:
//   #{ ShowMenu: #{ title: "+Menu", items: [ #{ key: "a", label: "Action" } ] } }
//   "HideMenu"
//
// This example inserts a divider comment on Ctrl-/ and otherwise stays out of
// the way, so normal typing is unaffected.

fn on_key() {
    if ctrl && key == "/" {
        ops.push("Consume");
        ops.push(#{ Insert: "// ----------------\n" });
    }
}
