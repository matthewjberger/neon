// Fireworks: random bursts of sparks that arc out and fade.
fn on_tick() {
    if !("sparks" in state) {
        state.sparks = [];
    }
    if random() < 0.03 {
        let cx = random_range(-4.0, 4.0);
        let cy = random_range(3.0, 6.0);
        let cz = random_range(-4.0, 4.0);
        let color = random_color();
        for index in 0..24 {
            let angle = index.to_float() / 24.0 * 6.2831853;
            state.sparks.push(#{
                x: cx, y: cy, z: cz,
                vx: cos(angle), vy: random_range(0.2, 1.6), vz: sin(angle),
                life: 1.5, c: color,
            });
        }
    }
    let next = [];
    for spark in state.sparks {
        spark.life -= dt;
        if spark.life > 0.0 {
            spark.vy -= dt * 3.0;
            spark.x += spark.vx * dt * 2.0;
            spark.y += spark.vy * dt * 2.0;
            spark.z += spark.vz * dt * 2.0;
            commands.draw_sphere([spark.x, spark.y, spark.z], 0.1, spark.c);
            next.push(spark);
        }
    }
    state.sparks = next;
}
