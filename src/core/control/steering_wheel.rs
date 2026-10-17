/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use log::{trace, warn};

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
        self.velocity.abs() < Self::SPEED_THRESHOLD
    }
}

impl SteeringWheel {
    /// rad/s
    const SPEED_THRESHOLD: f32 = 1e-5;

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
        if self.angle.abs() > half_range {
            self.angle = half_range * self.angle.signum();
            self.velocity = 0.0;
        }

        if !self.angle.is_finite() {
            self.angle = 0.0;
        }

        if !self.velocity.is_finite() || self.velocity.abs() < Self::SPEED_THRESHOLD {
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

        let mut dt_left = dt;

        let max_substeps = 15;
        for step in 0..max_substeps {
            if dt_left <= 0.0 {
                return;
            }

            // Calculate torques not accounting for friction yet.
            let spring_torque = config.spring_stiffness * -self.angle;
            let damping_torque = config.damping_coeff * -self.velocity;
            let torque_no_friction = ffb_torque + spring_torque + damping_torque;

            // Check if static friction applies. If so, we can skip this tick.
            if self.velocity.abs() <= f32::EPSILON
                && torque_no_friction.abs() <= config.static_friction_torque
            {
                self.velocity = 0.0;

                return;
            }

            let f_k_torque = config.kinetic_friction_torque * -self.velocity.signum();
            let net_torque = torque_no_friction + f_k_torque;

            let accel = if net_torque != 0.0 && config.inertia != 0.0 {
                net_torque / config.inertia
            } else {
                // Velocity is not affected this tick.

                return;
            };

            let next_velocity = self.velocity + accel * dt_left;

            // Once again check if static friction applies. If so, the wheel
            // stops here.
            if next_velocity.abs() < f32::EPSILON
                && torque_no_friction.abs() <= config.static_friction_torque
            {
                // The wheel moves no further, but the wheel needs to be advanced
                // up to the point at whicht eh velocity reached zero.
                self.angle += self.velocity * dt_left + accel * dt_left * dt_left * 0.5;
                self.velocity = 0.0;

                return;
            }

            // Check if velocity ever crossed zero, as we have to account for that.

            if self.velocity == 0.0
                || next_velocity == 0.0
                || self.velocity.signum() == next_velocity.signum()
            {
                // Velocity did not cross, so we can stop here.

                self.velocity = next_velocity;
                self.angle += self.velocity * dt;

                return;
            }

            let dt_at_zero_vel = -self.velocity / accel;

            // Integrate up to the point velocity becomes zero.
            // We let the next substep handle the rest.
            self.angle += next_velocity * dt_at_zero_vel;
            self.velocity = 0.0;

            dt_left -= dt_at_zero_vel;

            trace!(
                "sub #{step}: vel 0-xing @ dt_x: {} ms ({:5.2}% of remaining dt): {} -> {}, acc: {accel} rad/s^2",
                dt_at_zero_vel * 1000.0,
                (dt_at_zero_vel / dt_left) * 100.0,
                self.velocity,
                next_velocity
            );

            // Do another substep now that we're past the zero point.
        }

        if dt_left > f32::EPSILON {
            warn!(
                "Physics step took more than {max_substeps} substeps! Remaining delta-time was {} ms from the initial {} ms.",
                dt_left * 1000.0,
                dt * 1000.0,
            );
        }
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
