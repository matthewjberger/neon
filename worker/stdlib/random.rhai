// random: convenience over the runtime's seeded generator (random,
// random_range, random_int are always in scope).

fn random_color() {
    [random(), random(), random(), 1.0]
}

// A point in a box of half-size `extent`, sitting on or above the ground.
fn random_point(extent) {
    [
        random_range(-extent, extent),
        random_range(0.0, extent),
        random_range(-extent, extent),
    ]
}

fn random_pick(list) {
    let chosen = ();
    if list.len() > 0 {
        chosen = list[random_int(0, list.len() - 1)];
    }
    chosen
}
