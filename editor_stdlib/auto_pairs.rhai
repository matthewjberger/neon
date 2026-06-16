// Auto-close brackets and quotes, the way most editors do. In insert mode,
// typing an opener inserts the matching closer and leaves the caret between
// them. Other keys are untouched.

fn on_key() {
    if mode != "insert" {
        return;
    }
    let pair = "";
    if key == "(" { pair = "()"; }
    else if key == "[" { pair = "[]"; }
    else if key == "{" { pair = "{}"; }
    else if key == "\"" { pair = "\"\""; }
    else if key == "'" { pair = "''"; }
    else if key == "`" { pair = "``"; }
    if pair != "" {
        ops.push("Consume");
        ops.push(#{ Insert: pair });
        ops.push(#{ Move: -1 });
    }
}
