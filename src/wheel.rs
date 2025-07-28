use eframe::egui::Pos2;

use crate::{config::Config, device::Device, pen::Pen};

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
    ) -> bool {
        let pen = pen.unwrap_or_default();
        let mut significant_change = false;

        if self.angle != self.prev_angle
            && let Some(dev) = device.as_mut()
        {
            let normalised = self.angle / (config.range * 0.5);
            dev.set_wheel(normalised);
            significant_change = true;
        }

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

            let w = self.velocity.to_radians();
            let theta = self.angle.to_radians();

            let net_force = self.feedback_torque - config.friction * w - config.spring * theta;
            let acc = net_force / config.inertia;

            self.velocity += (acc * dt).to_degrees();

            if self.velocity.abs() < 0.05 {
                self.velocity = 0.0;
            }

            self.prev_angle = self.angle;
            self.angle += self.velocity * dt;

            if let Some(dev) = device.as_mut()
                && self.velocity.abs() > 0.01
            {
                let normalised = self.angle / (config.range * 0.5);
                dev.set_wheel(normalised);
                significant_change = true;
            }
        }

        self.angle = clamp_symmetric(config.range * 0.5, self.angle);

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

            return significant_change;
        }

        // wheel is held

        if self.honking {
            return significant_change;
        }

        let centre_dist = dist_sq(pen.x, pen.y).sqrt();

        if !self.dragging && centre_dist <= config.horn_radius {
            // start honking
            self.honking = true;
            if let Some(dev) = device {
                dev.set_horn(true);
            }

            return significant_change;
        }

        // check if we were already dragging
        if self.dragging {
            let prev_theta = self.prev_pos.x.atan2(self.prev_pos.y).to_degrees();
            let theta = pen.x.atan2(pen.y).to_degrees();

            let delta_t = angle_delta(prev_theta, theta);
            let adjusted = adjust_angle_delta(delta_t, centre_dist, config.base_radius);

            let new_angle = self.angle + adjusted;
            self.angle = clamp_symmetric(config.range * 0.5, new_angle);

            if let Some(dev) = device {
                let normalised = self.angle / (config.range * 0.5);
                dev.set_wheel(normalised);
                significant_change = true;
            }
        }

        self.dragging = true;
        self.prev_pos.x = pen.x;
        self.prev_pos.y = pen.y;

        significant_change
    }
}

fn dist_sq(x: f32, y: f32) -> f32 {
    x * x + y * y
}

fn clamp_symmetric(max_d: f32, v: f32) -> f32 {
    if v < -max_d {
        return -max_d;
    }

    if v > max_d {
        return max_d;
    }

    v
}

fn angle_delta(a: f32, b: f32) -> f32 {
    let mut delta = b - a;
    while delta < -180.0 {
        delta += 360.0;
    }

    while delta > 180.0 {
        delta -= 360.0;
    }

    delta
}

fn adjust_angle_delta(angle: f32, dist: f32, base: f32) -> f32 {
    let factor = dist.min(base) / base;
    angle * factor
}
