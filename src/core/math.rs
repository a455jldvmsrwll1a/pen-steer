use std::{
    f32::consts::PI,
    ops::{Add, Div, Mul, Sub},
};

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

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
