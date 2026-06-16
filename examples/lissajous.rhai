// A Lissajous curve traced in space, drifting with time.
fn on_tick() {
    let count = 200;
    for index in 0..count {
        let t = index.to_float() / count.to_float() * 6.2831853;
        let x = sin(t * 3.0 + time) * 4.0;
        let y = 2.0 + sin(t * 2.0) * 2.0;
        let z = sin(t * 4.0) * 4.0;
        commands.draw_sphere([x, y, z], 0.1, hsv(index.to_float() / count.to_float(), 0.7, 1.0));
    }
}
