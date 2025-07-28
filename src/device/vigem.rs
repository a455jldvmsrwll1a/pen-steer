use anyhow::{Context, Result};
use log::{error, info};
use vigem_client::{Client, TargetId, XButtons, XGamepad, Xbox360Wired};

use crate::device::Device;

pub struct VigemDevice {
    target: Xbox360Wired<Client>,
    last_angle: i16,
    last_horn_state: bool,
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
            last_angle: 0,
            last_horn_state: false,
            dirty: true,
        })
    }
}

impl Device for VigemDevice {
    fn get_feedback(&self) -> Option<f32> {
        None
    }

    fn set_wheel(&mut self, angle: f32) {
        let clamped = angle.clamp(-1.0, 1.0);
        let value = (clamped * i16::MAX as f32) as i16;

        if self.last_angle != value {
            self.last_angle = value;
            self.dirty = true;
        }
    }

    fn set_horn(&mut self, honking: bool) {
        if self.last_horn_state != honking {
            self.last_horn_state = honking;
            self.dirty = true;
        }
    }

    fn apply(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }

        let buttons = if self.last_horn_state {
            XButtons::LTHUMB.into()
        } else {
            XButtons::default()
        };

        self.target.update(&XGamepad {
            buttons,
            left_trigger: 0,
            right_trigger: 0,
            thumb_lx: self.last_angle,
            thumb_ly: 0,
            thumb_rx: 0,
            thumb_ry: 0,
        })?;

        Ok(())
    }

    fn handle_events(&mut self) {}
}

impl Drop for VigemDevice {
    fn drop(&mut self) {
        if let Err(err) = self.target.unplug() {
            error!("Could not unplug Vigem controller: {err}");
        }
    }
}
