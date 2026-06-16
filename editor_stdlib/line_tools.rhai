// Line tools, the VSCode-style line operations: move the current line up or
// down, duplicate it, delete it, and indent or outdent.

fn on_key() {
    if alt && key == "ArrowUp" {
        ops.push("Consume");
        ops.push("MoveLineUp");
    } else if alt && key == "ArrowDown" {
        ops.push("Consume");
        ops.push("MoveLineDown");
    } else if alt && shift && key == "ArrowDown" {
        ops.push("Consume");
        ops.push("DuplicateLine");
    } else if ctrl && shift && (key == "k" || key == "K") {
        ops.push("Consume");
        ops.push("DeleteLine");
    } else if ctrl && key == "]" {
        ops.push("Consume");
        ops.push("Indent");
    } else if ctrl && key == "[" {
        ops.push("Consume");
        ops.push("Outdent");
    }
}
