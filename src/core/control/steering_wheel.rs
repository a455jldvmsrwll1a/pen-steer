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
    velocity: f32,
    state: HoldState,
    horn_toggled: bool,
}

impl Default for SteeringWheel {
    fn default() -> Self {
        Self {
            angle: 0.0,
            velocity: 0.0,
            state: HoldState::Free,
            horn_toggled: false,
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
        let min_pressure = (config.pressure_min / 100.0).max(0.01);
        let horn_radius = config.horn_radius / 100.0;
        let base_radius = config.natural_rotation_radius_min / 100.0;
        let can_switch_anytime = config.fast_mode_switch;

        let can_drag = pen.pressure >= min_pressure
            && (!config.ignore_exterior_input || pen.is_within_range(..=1.0));

        // Handle state transitions.
        match &mut self.state {
            HoldState::Free => {
                if can_drag {
                    if pen.is_within_range(..=horn_radius) {
                        self.state = HoldState::PressingHorn;
                        self.horn_toggled = !self.horn_toggled;
                    } else {
                        self.state = HoldState::Dragging;
                    }
                }
            }
            HoldState::PressingHorn => {
                if !can_drag {
                    self.state = HoldState::Free;
                } else if can_switch_anytime && !pen.is_within_range(..=horn_radius) {
                    self.state = HoldState::Dragging;
                    self.horn_toggled = !self.horn_toggled;
                }
            }
            HoldState::Dragging => {
                if !can_drag {
                    self.state = HoldState::Free;
                } else if can_switch_anytime && pen.is_within_range(..=horn_radius) {
                    self.state = HoldState::PressingHorn;
                    self.horn_toggled = !self.horn_toggled;
                }
            }
        }

        match self.state {
            HoldState::Free | HoldState::PressingHorn => 'phys: {
                if !config.physics_enable {
                    self.velocity = 0.0;

                    break 'phys;
                }

                let ffb_torque = if config.enable_ffb
                    && let FFbState::Active { constant_force, .. } = force_feedback
                {
                    let sign = if config.full_ffb_negate { -1.0 } else { 1.0 };

                    // TODO: either `full_ffb_force` should be `full_ffb_torque`,
                    // or we gotta turn this into a torque somehow
                    constant_force * config.full_ffb_force * sign
                } else {
                    0.0
                };

                let friction_torque = config.friction_coeff * -self.velocity;
                let spring_torque = config.spring_stiffness * -self.angle;
                let net_force = ffb_torque + friction_torque + spring_torque;
                let angular_acceleration = net_force / config.inertia;

                self.velocity += angular_acceleration * dt;
                self.angle += self.velocity * dt;
            }
            HoldState::Dragging => {
                let angular_velocity = pen.compute_angular_velocity();
                let dtheta = angular_velocity * dt;
                let r = pen.distance_to_centre();
                let adjusted_delta = math::adjust_angle_delta(dtheta, r, base_radius);

                self.angle += adjusted_delta;
                self.velocity = angular_velocity;
            }
        }

        if !self.angle.is_finite() {
            self.angle = 0.0;
        }

        if !self.velocity.is_finite() || self.velocity.abs() < 1e-5 {
            self.velocity = 0.0;
        }

        let half_range = config.angle_range.to_radians() * 0.5;
        self.angle = math::clamp_symmetric(half_range, self.angle);

        // Positive angle is counterclockwise, so we need to take the negative of it.
        output.wheel = -self.angle / half_range;

        output.horn = if config.sticky_horn {
            self.horn_toggled
        } else {
            matches!(self.state, HoldState::PressingHorn)
        };

        output.accelerator = 0.0;
        output.brake = 0.0;
    }

    fn can_sleep(&self) -> bool {
        self.velocity.abs() < 1e-5
    }
}

#[derive(Debug, Clone)]
enum HoldState {
    Free,
    PressingHorn,
    Dragging,
}
