/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::core::{config::Profile, control::Control, math, output::OutputBackend, transform::Pen};

pub struct SteeringWheel {
    angle: f32,
    honking: bool,
    dragging: bool,
    prev_pen: Option<Pen>,
}

impl SteeringWheel {
    pub fn new(profile: &Profile) -> Self {
        Self {
            angle: 0.0,
            honking: false,
            dragging: false,
            prev_pen: None,
        }
    }
}

impl Control for SteeringWheel {
    fn update(
        &mut self,
        profile: &Profile,
        dt: f32,
        pen: Option<Pen>,
        mut output: Option<&mut (dyn OutputBackend + '_)>,
    ) {
        let config = &profile.steering_wheel_control;

        let pen_now = pen.filter(|p| p.pressure > config.pressure_min / 100.0);
        let pen_then = self
            .prev_pen
            .filter(|p| p.pressure > config.pressure_min / 100.0);

        self.prev_pen = pen;

        match (pen_now, pen_then) {
            (None, None) | (None, Some(..)) => {
                // Pen released.

                self.honking = false;
                self.dragging = false;
            }
            (Some(pen), None) => {
                // Pen just placed.

                if config.fast_mode_switch || (!self.dragging && !self.honking) {
                    let horn_radius = config.horn_radius / 100.0;
                    let r_sq = pen.position.length_squared();
                    if r_sq > horn_radius * horn_radius {
                        // We want to turn the wheel.
                        self.dragging = true;
                    } else {
                        // We want to press the horn.
                        self.honking = true;
                    }
                }
            }
            (Some(pen), Some(prev)) => {
                // Dragging / holding.

                if self.dragging {
                    let r = pen.position.length();

                    let dtheta =
                        if r > f32::EPSILON && prev.position.length_squared() > f32::EPSILON {
                            prev.position.angle_to(pen.position)
                        } else {
                            0.0
                        };

                    let base_radius = config.natural_rotation_radius_min / 100.0;
                    let adjusted_delta = math::adjust_angle_delta(dtheta, r, base_radius);
                    self.angle += adjusted_delta;

                    eprintln!(
                        "dtheta = {:3.0} deg, adjusted = {:3.0} deg, r = {:3.0}%",
                        dtheta.to_degrees(),
                        adjusted_delta.to_degrees(),
                        r * 100.0
                    );
                }
            }
        }

        if !self.angle.is_finite() {
            self.angle = 0.0;
        }

        let half_range = config.angle_range.to_radians() * 0.5;
        self.angle = math::clamp_symmetric(half_range, self.angle);

        // Positive angle is counterclockwise, so we need to take the negative of it.
        let angle_normalised = -self.angle / half_range;
        output.set_wheel(angle_normalised);
        output.set_horn(self.honking);
        output.set_accelerator(0.0);
        output.set_brake(0.0);

        eprintln!(
            "@ {:7.1} deg ({:4.0}%) [{}]",
            self.angle.to_degrees(),
            angle_normalised * 100.0,
            if self.honking { "HORN" } else { "    " }
        );
    }
}
