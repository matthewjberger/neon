//! The standard rhai library: procedural helpers plugins build on, embedded from
//! `worker/stdlib/*.rhai`. [`prelude`] is prepended to every enabled plugin so
//! the helpers are in scope. [`modules`] hands the page the source for the
//! in-editor viewer and the helper signatures for completion and the reference.

use protocol::{StdHelper, StdModule};

const SHAPES: &str = include_str!("../stdlib/shapes.rhai");
const COLOR: &str = include_str!("../stdlib/color.rhai");
const MOTION: &str = include_str!("../stdlib/motion.rhai");
const EVENTS: &str = include_str!("../stdlib/events.rhai");
const INPUT: &str = include_str!("../stdlib/input.rhai");
const RANDOM: &str = include_str!("../stdlib/random.rhai");

/// The whole library as one source blob, prepended to each plugin before it runs.
pub fn prelude() -> String {
    [SHAPES, COLOR, MOTION, EVENTS, INPUT, RANDOM].join("\n\n")
}

/// The library as modules, for the editor's source viewer and language service.
pub fn modules() -> Vec<StdModule> {
    vec![
        StdModule {
            name: "shapes".to_string(),
            source: SHAPES.to_string(),
            helpers: vec![
                helper("cube", "commands.cube(position, color)", "Spawn a cube and color it.", "commands"),
                helper("sphere", "commands.sphere(position, radius, color)", "Spawn a sphere and color it.", "commands"),
                helper("glowing", "commands.glowing(position, color, strength)", "Spawn an emissive sphere.", "commands"),
                helper("grid", "commands.grid(columns, rows, spacing, color)", "A centered grid of cubes.", "commands"),
                helper("ring", "commands.ring(count, radius, color)", "A ring of cubes in the xz plane.", "commands"),
            ],
        },
        StdModule {
            name: "color".to_string(),
            source: COLOR.to_string(),
            helpers: vec![
                helper("gray", "gray(level)", "An opaque gray.", "free"),
                helper("hsv", "hsv(h, s, v)", "HSV to RGBA, inputs 0..1.", "free"),
                helper("mix_color", "mix_color(a, b, t)", "Linear blend of two colors.", "free"),
            ],
        },
        StdModule {
            name: "motion".to_string(),
            source: MOTION.to_string(),
            helpers: vec![
                helper("spin", "commands.spin(entity, speed, dt)", "Spin about Y by speed * dt.", "commands"),
                helper("spin_axis", "commands.spin_axis(entity, axis, speed, dt)", "Spin about an axis.", "commands"),
                helper("bob", "commands.bob(entity, base_y, amplitude, frequency, time)", "Bob vertically.", "commands"),
                helper("orbit", "commands.orbit(entity, center, radius, speed, time)", "Move in a circle.", "commands"),
            ],
        },
        StdModule {
            name: "events".to_string(),
            source: EVENTS.to_string(),
            helpers: vec![
                helper("hits", "events.hits(self)", "Collisions starting this frame for self.", "events"),
                helper("sensor_hits", "events.sensor_hits(self)", "Sensor overlaps starting for self.", "events"),
                helper("other", "other(event, self)", "The other party in a collision.", "free"),
            ],
        },
        StdModule {
            name: "input".to_string(),
            source: INPUT.to_string(),
            helpers: vec![
                helper("axis_x", "axis_x(keys)", "Left/right axis from A/D or arrows.", "free"),
                helper("axis_z", "axis_z(keys)", "Forward/back axis from W/S or arrows.", "free"),
                helper("held", "held(keys, code)", "Whether a key code is held.", "free"),
            ],
        },
        StdModule {
            name: "random".to_string(),
            source: RANDOM.to_string(),
            helpers: vec![
                helper("random_color", "random_color()", "A random opaque color.", "free"),
                helper("random_point", "random_point(extent)", "A random point in a box.", "free"),
                helper("random_pick", "random_pick(list)", "A random element of a list.", "free"),
            ],
        },
    ]
}

fn helper(name: &str, signature: &str, description: &str, receiver: &str) -> StdHelper {
    StdHelper {
        name: name.to_string(),
        signature: signature.to_string(),
        description: description.to_string(),
        receiver: receiver.to_string(),
    }
}
