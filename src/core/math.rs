use std::f32::consts::PI;

/// Map `t` from range `a1..=a2` to range `b1..=b2`.
pub fn remap(t: f32, a1: f32, a2: f32, b1: f32, b2: f32) -> f32 {
    b1 + (t - a1) * (b2 - b1) / (a2 - a1)
}

/// Squared euclidean distance from (0, 0) to (`x`, `y`).
pub fn dist_sq(x: f32, y: f32) -> f32 {
    x * x + y * y
}

/// Clamp `v` within `-max_d..=max_d`.
pub fn clamp_symmetric(max_d: f32, v: f32) -> f32 {
    if v < -max_d {
        return -max_d;
    }

    if v > max_d {
        return max_d;
    }

    v
}

/// Shortest signed angular difference from `a` to `b` in radians.
pub fn angle_delta(a: f32, b: f32) -> f32 {
    let mut delta = b - a;
    while delta < -PI {
        delta += 2.0 * PI;
    }

    while delta > PI {
        delta -= 2.0 * PI;
    }

    delta
}

/// Adjust angle according to distance, up to a maximum.
pub fn adjust_angle_delta(angle: f32, dist: f32, base: f32) -> f32 {
    let factor = dist.min(base) / base;

    angle * factor
}
