// color: free helpers returning [r, g, b, a] arrays.

const RED = [0.9, 0.25, 0.2, 1.0];
const GREEN = [0.3, 0.8, 0.35, 1.0];
const BLUE = [0.25, 0.5, 0.95, 1.0];
const WHITE = [1.0, 1.0, 1.0, 1.0];

fn gray(level) {
    [level, level, level, 1.0]
}

// HSV to RGBA, all inputs in 0..1. The easy way to sweep hues procedurally.
fn hsv(h, s, v) {
    let sector = floor(h * 6.0);
    let frac = h * 6.0 - sector;
    let p = v * (1.0 - s);
    let q = v * (1.0 - frac * s);
    let t = v * (1.0 - (1.0 - frac) * s);
    let index = sector.to_int() % 6;
    let out = [v, p, q, 1.0];
    if index == 0 { out = [v, t, p, 1.0]; }
    if index == 1 { out = [q, v, p, 1.0]; }
    if index == 2 { out = [p, v, t, 1.0]; }
    if index == 3 { out = [p, q, v, 1.0]; }
    if index == 4 { out = [t, p, v, 1.0]; }
    out
}

// Linear blend between two colors, t in 0..1.
fn mix_color(a, b, t) {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}
