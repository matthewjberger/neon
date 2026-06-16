// motion: per-frame movement helpers on `commands`. Each takes the time inputs
// explicitly, because rhai functions do not capture the script's scope.

// Spin an entity about the Y axis. Pass the frame `dt`.
fn spin(entity, speed, dt) {
    this.rotate(entity, [0.0, 1.0, 0.0], speed * dt);
}

// Spin about an arbitrary axis.
fn spin_axis(entity, axis, speed, dt) {
    this.rotate(entity, axis, speed * dt);
}

// Bob an entity vertically around base_y. Pass the script `time`.
fn bob(entity, base_y, amplitude, frequency, time) {
    let y = base_y + sin(time * frequency) * amplitude;
    this.set_position(entity, [0.0, y, 0.0]);
}

// Move an entity in a circle around center in the xz plane. Pass `time`.
fn orbit(entity, center, radius, speed, time) {
    let angle = time * speed;
    this.set_position(entity, [
        center[0] + cos(angle) * radius,
        center[1],
        center[2] + sin(angle) * radius,
    ]);
}
