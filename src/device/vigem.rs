use anyhow::{Context, Result};
use log::{error, info};
use vigem_client::{Client, TargetId, XButtons, XGamepad, Xbox360Wired};

use crate::device::Device;

pub struct VigemDevice {
    target: Xbox360Wired<Client>,
    wheel_axis: i16,
    wheel_axis_prev: i16,
    accelerator_axis: u8,
    accelerator_axis_prev: u8,
    brake_axis: u8,
    brake_axis_prev: u8,
    horn_key: bool,
    horn_key_prev: bool,
    dirty: bool,
}

impl VigemDevice {
    pub fn new() -> Result<Self> {
        info!("Vigem device initialised!");

        let client = Client::connect()?;
        let mut target = Xbox360Wired::new(client, TargetId::XBOX360_WIRED);

        target
            .plugin()
            .context("Failed to connect Vigem controller.")?;
        target.wait_ready()?;

        Ok(Self {
            target,
            wheel_axis: 0,
            wheel_axis_prev: 0,
            accelerator_axis: 0,
            accelerator_axis_prev: 0,
            brake_axis: 0,
            brake_axis_prev: 0,
            horn_key: false,
            horn_key_prev: false,
            dirty: false,
        })
    }
}

impl Device for VigemDevice {
    fn get_feedback(&self) -> Option<f32> {
        None
    }

    #[allow(clippy::cast_possible_truncation)]
    fn set_wheel(&mut self, angle: f32) {
        let value = (angle * i16::MAX as f32).round_ties_even();
        self.wheel_axis = value as i16;

        if self
            .wheel_axis
            .checked_sub(self.wheel_axis_prev)
            .is_none_or(|delta| delta.abs() > 1)
        {
            self.wheel_axis_prev = self.wheel_axis;
            self.dirty = true;
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn set_accelerator(&mut self, normalised: f32) {
        let value = (normalised.clamp(0.0, 1.0) * u8::MAX as f32).round_ties_even();
        self.accelerator_axis = value as u8;

        if self.accelerator_axis != self.accelerator_axis_prev {
            self.accelerator_axis_prev = self.accelerator_axis;
            self.dirty = true;
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn set_brake(&mut self, normalised: f32) {
        let value = (normalised.clamp(0.0, 1.0) * u8::MAX as f32).round_ties_even();
        self.brake_axis = value as u8;

        if self.brake_axis != self.brake_axis_prev {
            self.brake_axis_prev = self.brake_axis;
            self.dirty = true;
        }
    }

    fn set_horn(&mut self, honking: bool) {
        self.horn_key = honking;

        if self.horn_key != self.horn_key_prev {
            self.horn_key_prev = self.horn_key;
            self.dirty = true;
        }
    }

    fn apply(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }

        self.dirty = false;

        let buttons = if self.horn_key {
            XButtons::LTHUMB.into()
        } else {
            XButtons::default()
        };

        self.target.update(&XGamepad {
            buttons,
            left_trigger: self.brake_axis,
            right_trigger: self.accelerator_axis,
            thumb_lx: self.wheel_axis,
            thumb_ly: 0,
            thumb_rx: 0,
            thumb_ry: 0,
        })?;

        Ok(())
    }

    fn handle_events(&mut self) {
        // No events to handle.
    }
}

impl Drop for VigemDevice {
    fn drop(&mut self) {
        if let Err(err) = self.target.unplug() {
            error!("Could not unplug Vigem controller: {err}");
        }
    }
}
