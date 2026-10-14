/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::ops::RangeBounds;

use glam::Vec2;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Pen {
    pub buttons: u32,
    pub pressure: f32,
    pub position: Vec2,
    pub velocity: Vec2,
}

impl Pen {
    pub fn distance_to_centre(&self) -> f32 {
        self.position.length()
    }

    pub fn is_within_range(&self, range: impl RangeBounds<f32>) -> bool {
        let start_sq = range.start_bound().map(|x| x * x);
        let end_sq = range.end_bound().map(|x| x * x);
        let r_sq = self.position.length_squared();

        (start_sq, end_sq).contains(&r_sq)
    }

    pub fn compute_angular_velocity(&self) -> f32 {
        let Vec2 { x, y } = self.position;
        let Vec2 { x: vx, y: vy } = self.velocity;

        let len_sq = self.position.length_squared();

        // undefined when at pen is at (0, 0)
        if len_sq < f32::EPSILON {
            return 0.0;
        }

        ((x * vy) - (y * vx)) / len_sq
    }
}
