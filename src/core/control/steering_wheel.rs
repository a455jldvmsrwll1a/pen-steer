/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::core::{config::Profile, control::Control, output::OutputBackend, transform::Pen};

pub struct SteeringWheel {}

impl SteeringWheel {
    pub fn new(profile: &Profile) -> Self {
        Self {}
    }
}

impl Control for SteeringWheel {
    fn update(
        &mut self,
        profile: &Profile,
        dt: f32,
        pen: Option<Pen>,
        output: Option<&mut dyn OutputBackend>,
    ) {
    }
}
