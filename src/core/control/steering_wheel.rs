/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::core::{config::Profile, control::Control, output::OutputBackend, transform::Pen};

pub struct SteeringWheel {
    acc: f32,
}

impl SteeringWheel {
    pub fn new(profile: &Profile) -> Self {
        Self {
            acc: 0.0,
        }
    }
}

impl Control for SteeringWheel {
    fn update(
        &mut self,
        profile: &Profile,
        dt: f32,
        pen: Option<Pen>,
        output: Option<&mut (dyn OutputBackend + '_)>,
    ) {
        self.acc += dt;

        eprintln!("{:.1} : ", self.acc);
    }
}
