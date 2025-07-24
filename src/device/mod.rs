#[cfg(target_os = "linux")]
pub mod uinput;

use crate::config;
#[cfg(target_os = "linux")]
use crate::device::uinput::UInputDevice;

use anyhow::Result;

pub trait Device: Send + Sync {
    fn get_feedback(&self) -> f32;

    fn set_wheel(&mut self, angle: f32);

    fn set_horn(&mut self, honking: bool);

    fn apply(&mut self) -> Result<()>;

    fn handle_events(&mut self);
}

pub struct DummyDevice;

impl Device for DummyDevice {
    fn get_feedback(&self) -> f32 {
        0.0
    }

    fn set_wheel(&mut self, _angle: f32) {}

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
        config::Device::VigemBus => Box::new(DummyDevice),
    })
}
