// input: read the held-key map `keys` into axes. Pass `keys` from the script.

// -1 for left (A or Left), +1 for right (D or Right), 0 otherwise.
fn axis_x(keys) {
    let value = 0.0;
    if ("KeyA" in keys) || ("ArrowLeft" in keys) { value -= 1.0; }
    if ("KeyD" in keys) || ("ArrowRight" in keys) { value += 1.0; }
    value
}

// -1 for forward (W or Up), +1 for back (S or Down), 0 otherwise.
fn axis_z(keys) {
    let value = 0.0;
    if ("KeyW" in keys) || ("ArrowUp" in keys) { value -= 1.0; }
    if ("KeyS" in keys) || ("ArrowDown" in keys) { value += 1.0; }
    value
}

fn held(keys, code) {
    code in keys
}
