// TEMPLATE editor plugin: handle keystrokes in the code editor.
//
// on_key() runs for every keystroke. In scope:
//   key                  the key name ("a", "Enter", "Escape", "ArrowLeft", ...)
//   mode                 the current editor mode label
//   ctrl, shift, alt     modifier booleans
//   ops                  push actions here
//   state                a map that persists across keystrokes
//
// Push to ops:
//   "Consume"                       stop the key from typing normally
//   #{ SetMode: "insert" }          set the mode label
//   #{ Insert: "text" }             insert text at the cursor
//   #{ Move: 1 } / #{ MoveLine: 1 } move the cursor (negative reverses)
//   "LineStart" "LineEnd" "NextWord" "PrevWord"
//   #{ DeleteForward: 1 } #{ DeleteBackward: 1 } "DeleteLine"
//   #{ SetStatus: "..." }
//
// This example inserts a divider comment on Ctrl-/ and otherwise stays out of
// the way, so normal typing is unaffected.

fn on_key() {
    if ctrl && key == "/" {
        ops.push("Consume");
        ops.push(#{ Insert: "// ----------------\n" });
    }
}
