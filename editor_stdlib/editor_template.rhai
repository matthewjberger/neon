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
//   #{ ToggleComment: "//" }        comment or uncomment the line
//   #{ FindChar: "x" }              jump to the next x on the line
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
