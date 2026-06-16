// events: filters over this frame's `events` array. Called as methods on it,
// e.g. for hit in events.hits(self) { ... }

// Collisions that started this frame and involve self_ref.
fn hits(self_ref) {
    let out = [];
    for event in this {
        if event.kind == "collision" && event.started {
            if event.a == self_ref || event.b == self_ref {
                out.push(event);
            }
        }
    }
    out
}

// Sensor overlaps that started this frame and involve self_ref.
fn sensor_hits(self_ref) {
    let out = [];
    for event in this {
        if event.kind == "collision" && event.started && event.sensor {
            if event.a == self_ref || event.b == self_ref {
                out.push(event);
            }
        }
    }
    out
}

// The other party in a collision event, given self.
fn other(event, self_ref) {
    if event.a == self_ref { event.b } else { event.a }
}
