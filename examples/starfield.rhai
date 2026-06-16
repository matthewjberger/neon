// A warp starfield: stars stream toward the camera and recycle.
fn on_tick() {
    if !("stars" in state) {
        state.stars = [];
        for index in 0..80 {
            state.stars.push(#{
                x: random_range(-6.0, 6.0),
                y: random_range(-3.0, 4.0),
                z: random_range(0.5, 20.0),
            });
        }
    }
    let next = [];
    for star in state.stars {
        star.z -= dt * 8.0;
        if star.z < 0.5 {
            star.z += 20.0;
        }
        commands.draw_sphere([star.x, star.y + 1.0, star.z], 0.1, WHITE);
        next.push(star);
    }
    state.stars = next;
}
