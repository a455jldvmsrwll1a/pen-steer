/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

pub mod steering_wheel;

use crate::core::{
    config::Profile,
    control::steering_wheel::SteeringWheel,
    output::{FFbState, OutputSample},
    pen::Pen,
};

pub trait Control {
    fn update(
        &mut self,
        profile: &Profile,
        dt: f32,
        pen: Pen,
        force_feedback: &FFbState,
        output: &mut OutputSample,
    );

    fn can_sleep(&self) -> bool;
}

pub fn create_control(profile: &Profile) -> Box<dyn Control> {
    match profile.active_control {
        super::config::ControlType::Dummy => Box::new(DummyControl::default()),
        super::config::ControlType::SteeringWheel => Box::new(SteeringWheel::default()),
        super::config::ControlType::OneTabletDrive => todo!("one-tablet-drive under construction"),
    }
}

#[derive(Debug, Default)]
pub struct DummyControl;

impl Control for DummyControl {
    fn update(&mut self, _: &Profile, _: f32, _: Pen, _: &FFbState, _: &mut OutputSample) {}

    fn can_sleep(&self) -> bool {
        true
    }
}
