// Move lines with Alt+j and Alt+k, the home-row way to shuffle a line up or
// down. Works in any mode.

fn on_key() {
    if alt && key == "j" {
        ops.push("Consume");
        ops.push("MoveLineDown");
    } else if alt && key == "k" {
        ops.push("Consume");
        ops.push("MoveLineUp");
    }
}
