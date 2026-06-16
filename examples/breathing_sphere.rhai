// A single sphere that breathes, its radius and hue easing with time.
fn on_tick() {
    let radius = 1.0 + sin(time * 1.5) * 0.4;
    let hue = (sin(time * 0.3) + 1.0) * 0.5;
    commands.draw_sphere([0.0, 1.5, 0.0], radius, hsv(hue, 0.7, 1.0));
}
