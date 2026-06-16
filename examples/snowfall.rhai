// Gentle snowfall that drifts sideways as it falls.
fn on_tick() {
    if !("flakes" in state) {
        state.flakes = [];
    }
    if random() < 0.5 {
        state.flakes.push(#{
            x: random_range(-7.0, 7.0),
            y: 10.0,
            z: random_range(-7.0, 7.0),
        });
    }
    let next = [];
    for flake in state.flakes {
        flake.y -= dt * 2.0;
        flake.x += sin(time + flake.y) * dt * 0.5;
        if flake.y > 0.0 {
            commands.draw_sphere([flake.x, flake.y, flake.z], 0.12, WHITE);
            next.push(flake);
        }
    }
    state.flakes = next;
}
