// Balls dropped under gravity, bouncing off the floor with some damping.
fn on_tick() {
    if !("balls" in state) {
        state.balls = [];
        for index in 0..8 {
            state.balls.push(#{
                x: random_range(-4.0, 4.0),
                y: random_range(2.0, 6.0),
                z: random_range(-4.0, 4.0),
                vy: 0.0,
                c: random_color(),
            });
        }
    }
    let next = [];
    for ball in state.balls {
        ball.vy -= dt * 9.8;
        ball.y += ball.vy * dt;
        if ball.y < 0.3 {
            ball.y = 0.3;
            ball.vy = -ball.vy * 0.8;
        }
        commands.draw_sphere([ball.x, ball.y, ball.z], 0.3, ball.c);
        next.push(ball);
    }
    state.balls = next;
}
