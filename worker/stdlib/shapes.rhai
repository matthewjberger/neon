// shapes: builders that spawn and color geometry in one call.
// Called as methods on `commands`, e.g. commands.cube([0.0, 0.5, 0.0], RED);

fn cube(position, color) {
    this.spawn_cube(position);
    this.set_color(this.last(), color);
}

fn sphere(position, radius, color) {
    this.spawn_sphere(position);
    let entity = this.last();
    this.set_color(entity, color);
    this.set_scale(entity, [radius * 2.0, radius * 2.0, radius * 2.0]);
}

fn glowing(position, color, strength) {
    this.spawn_sphere(position);
    let entity = this.last();
    this.set_color(entity, color);
    this.set_emissive(entity, [color[0], color[1], color[2]], strength);
}

// A flat columns x rows grid of cubes centered on the origin.
fn grid(columns, rows, spacing, color) {
    let span_x = columns - 1;
    let span_z = rows - 1;
    let half_x = span_x.to_float() * spacing * 0.5;
    let half_z = span_z.to_float() * spacing * 0.5;
    for column in 0..columns {
        for row in 0..rows {
            let x = column.to_float() * spacing - half_x;
            let z = row.to_float() * spacing - half_z;
            this.cube([x, 0.5, z], color);
        }
    }
}

// A ring of cubes in the xz plane.
fn ring(count, radius, color) {
    let step = 6.2831853 / count.to_float();
    for index in 0..count {
        let angle = index.to_float() * step;
        this.cube([cos(angle) * radius, 0.5, sin(angle) * radius], color);
    }
}
