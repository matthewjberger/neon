// A grid of spheres rippling like water, redrawn each frame.
fn on_tick() {
    let size = 9;
    for column in 0..size {
        for row in 0..size {
            let x = column.to_float() - 4.0;
            let z = row.to_float() - 4.0;
            let y = 1.5 + sin(x * 0.8 + time * 2.0) * cos(z * 0.8 + time * 1.5);
            let hue = (x + z) / 18.0 + time * 0.1;
            commands.draw_sphere([x, y, z], 0.25, hsv(hue, 0.7, 1.0));
        }
    }
}
