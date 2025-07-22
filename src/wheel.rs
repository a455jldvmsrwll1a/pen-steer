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
}

impl Wheel {
    pub fn update(
        &mut self,
        device: Option<&mut Device>,
        config: &Config,
        pen: Option<Pen>,
        dt: f32,
    ) {
        let pen = pen.unwrap_or_default();

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

            return;
        }

        // wheel is held

        if self.honking {
            return;
        }

        let centre_dist = dist_sq(pen.x, pen.y).sqrt();

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
            let prev_theta = self.prev_pos.x.atan2(self.prev_pos.y).to_degrees();
            let theta = pen.x.atan2(pen.y).to_degrees();

            let delta_t = angle_delta(prev_theta, theta);
            let adjusted = adjust_angle_delta(delta_t, centre_dist, config.base_radius);

            let new_angle = self.angle + adjusted;
            self.angle = clamp_symmetric(config.range * 0.5, new_angle);

            let normalised = self.angle / (config.range * 0.5);
            if let Some(dev) = device {
                dev.set_wheel(normalised);
            }
        }

        self.dragging = true;
        self.prev_pos.x = pen.x;
        self.prev_pos.y = pen.y;
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
