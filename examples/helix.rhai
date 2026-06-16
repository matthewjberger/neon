// A rotating double helix, two strands a half-turn apart.
fn on_tick() {
    let count = 40;
    for index in 0..count {
        let t = index.to_float() / count.to_float();
        let y = t * 6.0;
        let angle = t * 12.0 + time * 1.5;
        commands.draw_sphere([cos(angle) * 1.5, y, sin(angle) * 1.5], 0.15, hsv(0.6, 0.7, 1.0));
        let opposite = angle + 3.1415927;
        commands.draw_sphere([cos(opposite) * 1.5, y, sin(opposite) * 1.5], 0.15, hsv(0.95, 0.7, 1.0));
    }
}
