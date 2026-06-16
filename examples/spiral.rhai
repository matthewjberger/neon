// A rotating spiral arm climbing upward.
fn on_tick() {
    let count = 120;
    for index in 0..count {
        let t = index.to_float() / count.to_float();
        let angle = t * 18.0 + time;
        let radius = t * 5.0;
        commands.draw_sphere([cos(angle) * radius, t * 4.0, sin(angle) * radius], 0.12, hsv(t, 0.7, 1.0));
    }
}
