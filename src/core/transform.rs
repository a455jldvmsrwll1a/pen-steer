/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::time::{Duration, Instant};

use glam::{Affine2, Vec2};

use crate::core::{config::Mapping, input::InputSample};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Pen {
    pub buttons: u32,
    pub pressure: f32,
    pub position: Vec2,
    pub velocity: Vec2,
}

pub struct Transformer {
    transform: Affine2,
    pen: Pen,
    prev_time: Instant,
}

impl Transformer {
    pub fn new(mapping: &Mapping) -> Self {
        let converter: Affine2 =
            Affine2::from_scale_angle_translation(Vec2::splat(2.0), 0.0, Vec2::splat(-1.0));

        let user_transform = Affine2::from_scale_angle_translation(
            mapping.scale,
            mapping.rotation,
            mapping.translation,
        );

        Self {
            transform: converter * user_transform,
            pen: Pen::default(),
            prev_time: Instant::now(),
        }
    }

    pub fn update(&mut self, input: InputSample) {
        const MIN_DT: Duration = Duration::from_micros(1);
        const MAX_DT: Duration = Duration::from_millis(500);

        let dt = input.timestamp - self.prev_time;
        self.prev_time = input.timestamp;

        let normalised_position = self.transform.transform_point2(input.coordinate);
        let position = normalised_position * Vec2::new(input.aspect_ratio, 1.0);

        let velocity = if dt < MIN_DT || dt > MAX_DT {
            Vec2::ZERO
        } else {
            (position - self.pen.position) / dt.as_secs_f32()
        };

        self.pen = Pen {
            buttons: input.buttons,
            pressure: input.pressure,
            position,
            velocity,
        };
    }

    pub fn get_pen(&self) -> Pen {
        self.pen
    }
}
