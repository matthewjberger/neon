// Word deletion, the chord most editors use: Ctrl+Backspace deletes the word
// before the caret, Ctrl+Delete deletes the word after it.

fn on_key() {
    if ctrl && key == "Backspace" {
        ops.push("Consume");
        ops.push("DeleteWordBackward");
    } else if ctrl && key == "Delete" {
        ops.push("Consume");
        ops.push("DeleteWordForward");
    }
}
