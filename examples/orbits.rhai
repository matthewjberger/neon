// A little solar system: rings of bodies orbiting a glowing center.
fn on_tick() {
    commands.draw_sphere([0.0, 1.5, 0.0], 0.6, [1.0, 0.9, 0.3, 1.0]);
    let count = 5;
    let step = 6.2831853 / count.to_float();
    for index in 0..count {
        let radius = 1.6 + index.to_float() * 0.9;
        let speed = 1.4 - index.to_float() * 0.2;
        let angle = time * speed + index.to_float() * step;
        let hue = index.to_float() / count.to_float();
        commands.draw_sphere([cos(angle) * radius, 1.5, sin(angle) * radius], 0.3, hsv(hue, 0.7, 1.0));
    }
}
