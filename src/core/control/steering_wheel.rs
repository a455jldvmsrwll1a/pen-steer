/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::core::{
    config::{Profile, SteeringWheelControl},
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

        self.handle_state_transitions(&pen, config);

        match self.state {
            HoldState::Free | HoldState::PressingHorn => {
                self.run_physics(force_feedback, config, dt);
            }
            HoldState::Dragging => {
                self.when_dragging_directly(&pen, config, dt);
            }
        }

        self.fix_up(config);
        self.apply_output(output, config);
    }

    fn can_sleep(&self) -> bool {
        self.velocity.abs() < 1e-5
    }
}

impl SteeringWheel {
    fn apply_output(&self, output: &mut OutputSample, config: &SteeringWheelControl) {
        let half_range = config.angle_range.to_radians().abs() * 0.5;

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

    fn fix_up(&mut self, config: &SteeringWheelControl) {
        let half_range = config.angle_range.to_radians().abs() * 0.5;
        self.angle = self.angle.clamp(-half_range, half_range);

        if !self.angle.is_finite() {
            self.angle = 0.0;
        }

        if !self.velocity.is_finite() || self.velocity.abs() < 1e-5 {
            self.velocity = 0.0;
        }
    }

    fn when_dragging_directly(&mut self, pen: &Pen, config: &SteeringWheelControl, dt: f32) {
        let base_radius = config.natural_rotation_radius_min / 100.0;

        let angular_velocity = pen.compute_angular_velocity();
        let dtheta = angular_velocity * dt;
        let r = pen.distance_to_centre();
        let adjusted_delta = math::adjust_angle_delta(dtheta, r, base_radius);

        self.angle += adjusted_delta;
        self.velocity = angular_velocity;
    }

    fn run_physics(&mut self, force_feedback: &FFbState, config: &SteeringWheelControl, dt: f32) {
        if !config.physics_enable {
            self.velocity = 0.0;

            return;
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

        // Calculate torques not accounting for friction yet.
        let spring_torque = config.spring_stiffness * -self.angle;
        let damping_torque = config.damping_coeff * -self.velocity;
        let torque_no_friction = ffb_torque + spring_torque + damping_torque;

        // Apply static or kinetic friction depending on current situation.
        // TODO: fix issues that can happen when velocity crosses zero or goes near it in a tick.
        let friction_torque = if self.velocity.abs() >= 1e-5
            || torque_no_friction.abs() > config.static_friction_torque
        {
            // FIXME: KINETIC HINDI KINEMATIC 8080
            config.kinetic_friction_torque * -self.velocity.signum()
        } else {
            math::clamp_symmetric(config.static_friction_torque, -torque_no_friction)
        };

        let net_force = torque_no_friction + friction_torque;
        let angular_acceleration = net_force / config.inertia;

        self.velocity += angular_acceleration * dt;
        self.angle += self.velocity * dt;
    }

    fn handle_state_transitions(&mut self, pen: &Pen, config: &SteeringWheelControl) {
        let min_pressure = (config.pressure_min / 100.0).max(0.01);
        let horn_radius = config.horn_radius / 100.0;
        let can_switch_anytime = config.fast_mode_switch;

        let can_drag = pen.pressure >= min_pressure
            && (!config.ignore_exterior_input || pen.is_within_range(..=1.0));

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
    }
}

#[derive(Debug, Clone)]
enum HoldState {
    Free,
    PressingHorn,
    Dragging,
}
