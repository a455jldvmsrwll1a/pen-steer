/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::core::{
    config::Profile,
    control::Control,
    math,
    output::{FFbState, OutputSample},
    pen::Pen,
};

pub struct SteeringWheel {
    angle: f32,
    state: HoldState,
}

impl Default for SteeringWheel {
    fn default() -> Self {
        Self {
            angle: 0.0,
            state: HoldState::Free,
        }
    }
}

impl Control for SteeringWheel {
    fn update(
        &mut self,
        profile: &Profile,
        dt: f32,
        pen: Pen,
        force_feedback: &FFbState,
        output: &mut OutputSample,
    ) {
        let config = &profile.steering_wheel_control;
        let min_pressure = config.pressure_min / 100.0;
        let horn_radius = config.horn_radius / 100.0;
        let base_radius = config.natural_rotation_radius_min / 100.0;

        match &mut self.state {
            HoldState::Free => {
                if pen.pressure >= min_pressure {
                    if pen.is_within_range(..=horn_radius) {
                        self.state = HoldState::Honking;
                    } else {
                        self.state = HoldState::Dragging;
                    }
                }
            }
            HoldState::Honking => {
                if pen.pressure < min_pressure {
                    self.state = HoldState::Free;
                } else if config.fast_mode_switch && !pen.is_within_range(..=horn_radius) {
                    self.state = HoldState::Dragging;
                }
            }
            HoldState::Dragging => {
                if pen.pressure < min_pressure {
                    self.state = HoldState::Free;
                } else if config.fast_mode_switch && pen.is_within_range(..=horn_radius) {
                    self.state = HoldState::Honking;
                }

                let dtheta = pen.compute_angular_velocity() * dt;
                let r = pen.distance_to_centre();
                let adjusted_delta = math::adjust_angle_delta(dtheta, r, base_radius);

                self.angle += adjusted_delta;
            }
        }

        if !self.angle.is_finite() {
            self.angle = 0.0;
        }

        let half_range = config.angle_range.to_radians() * 0.5;
        self.angle = math::clamp_symmetric(half_range, self.angle);

        // Positive angle is counterclockwise, so we need to take the negative of it.
        output.wheel = -self.angle / half_range;
        output.horn = matches!(self.state, HoldState::Honking);
        output.accelerator = 0.0;
        output.brake = 0.0;
    }
}

#[derive(Debug, Clone)]
enum HoldState {
    Free,
    Honking,
    Dragging,
}
