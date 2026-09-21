#[cfg(target_os = "linux")]
pub mod uinput;

#[cfg(target_os = "windows")]
pub mod vigem;

use crate::config;
#[cfg(target_os = "linux")]
use crate::device::uinput::UInputDevice;
#[cfg(target_os = "windows")]
use crate::device::vigem::VigemDevice;

use anyhow::Result;

/// Backend for the output device.
/// 
pub trait Device: Send + Sync {
    /// Get the current feedback force from the output device.
    /// 
    /// # Returns
    /// 
    /// The feedback force (in no particular unit), or `None` if unavailable:
    /// 
    /// - The output backend or device does not support force-feedback.
    /// 
    /// - Force-feedback is currently not being used by any process.
    /// 
    fn get_feedback(&self) -> Option<f32>;

    /// Set the wheel axis value.
    /// 
    /// Full range is [-1.0, 1.0], in no particular unit.
    /// 
    fn set_wheel(&mut self, angle: f32);

    /// Set the accelerator axis value.
    /// 
    /// Full range is [0.0, 1.0], in no particular unit.
    /// 
    fn set_accelerator(&mut self, normalised: f32);

    /// Set the brake axis value.
    /// 
    /// Full range is [0.0, 1.0], in no particular unit.
    /// 
    fn set_brake(&mut self, normalised: f32);

    /// Set the horn's state.
    /// 
    /// `true` if the horn is pressed, and `false` if not.
    /// 
    fn set_horn(&mut self, honking: bool);

    /// Apply any changes that were made to the actual output mechanism (e.g. uinput or vigem).
    /// 
    fn apply(&mut self) -> Result<()>;

    /// Handle any events that the backend needs to respond to (such as receiving force-feedback events).
    /// 
    fn handle_events(&mut self);
}

pub struct DummyDevice;

impl Device for DummyDevice {
    fn get_feedback(&self) -> Option<f32> {
        None
    }

    fn set_wheel(&mut self, _angle: f32) {}

    fn set_accelerator(&mut self, _normalised: f32) {}

    fn set_brake(&mut self, _normalised: f32) {}

    fn set_horn(&mut self, _honking: bool) {}

    fn apply(&mut self) -> Result<()> {
        Ok(())
    }

    fn handle_events(&mut self) {}
}

pub fn create_device(config: &config::Config) -> Result<Box<dyn Device>> {
    Ok(match config.device {
        config::Device::None => Box::new(DummyDevice),
        #[cfg(target_os = "linux")]
        config::Device::UInput => Box::new(UInputDevice::new(config)?),
        #[cfg(target_os = "windows")]
        config::Device::VigemBus => Box::new(VigemDevice::new()?),
    })
}
