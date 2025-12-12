use eframe::egui::Pos2;

use crate::{config::Config, device::Device, math, pen::Pen};

#[derive(Debug, Default, Clone)]
pub struct Wheel {
    pub angle: f32,
    pub velocity: f32,
    pub feedback_torque: f32,
    pub honking: bool,
    pub dragging: bool,
    pub prev_pos: Pos2,
    pub prev_angle: f32,
}

impl Wheel {
    pub fn update(
        &mut self,
        mut device: Option<&mut Box<dyn Device>>,
        config: &Config,
        pen: Option<Pen>,
        dt: f32,
    ) {
        let half_range = config.half_range_rad();

        let pen = pen.unwrap_or_default();

        if self.velocity.is_nan() || self.velocity.is_infinite() {
            self.velocity = 0.0;
        }

        if self.angle.is_nan() || self.angle.is_infinite() {
            self.angle = 0.0;
        }

        if !self.dragging {
            let feedback_normalised = device
                .as_ref()
                .map(|d| d.get_feedback())
                .flatten()
                .unwrap_or(0.0);
            self.feedback_torque = feedback_normalised * config.max_torque;

            let friction_torque = config.friction * self.velocity;
            let spring_torque = config.spring * self.angle;
            let net_force = self.feedback_torque - friction_torque - spring_torque;
            let angular_acceleration = net_force / config.inertia;

            self.velocity += angular_acceleration * dt;

            if self.velocity.abs() < 1e-5 {
                self.velocity = 0.0;
            }

            self.prev_angle = self.angle;
            self.angle += self.velocity * dt;

            if let Some(dev) = device.as_mut() {
                let normalised = self.angle / half_range;
                dev.set_wheel(normalised);
            }
        }

        self.angle = math::clamp_symmetric(half_range, self.angle);

        // check if pen up
        if pen.pressure <= config.pressure_threshold {
            // stop honking
            if self.honking {
                if let Some(dev) = device {
                    dev.set_horn(false);
                }
            }

            self.honking = false;
            self.dragging = false;

            return;
        }

        // wheel is held

        if self.honking {
            return;
        }

        let centre_dist = math::dist_sq(pen.x, pen.y).sqrt();

        if !self.dragging && centre_dist <= config.horn_radius {
            // start honking
            self.honking = true;
            if let Some(dev) = device {
                dev.set_horn(true);
            }

            return;
        }

        // check if we were already dragging
        if self.dragging {
            let prev_theta = self.prev_pos.x.atan2(self.prev_pos.y);
            let theta = pen.x.atan2(pen.y);

            let delta_t = math::angle_delta(prev_theta, theta);
            let adjusted = math::adjust_angle_delta(delta_t, centre_dist, config.base_radius);

            let new_angle = self.angle + adjusted;
            self.prev_angle = self.angle;
            self.angle = math::clamp_symmetric(half_range, new_angle);

            self.velocity = (self.angle - self.prev_angle) / dt;

            if let Some(dev) = device {
                let normalised = self.angle / half_range;
                dev.set_wheel(normalised);
            }
        }

        self.dragging = true;
        self.prev_pos.x = pen.x;
        self.prev_pos.y = pen.y;
    }
}
