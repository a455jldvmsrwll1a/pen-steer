#[cfg(target_os = "linux")]
pub mod uinput;

#[cfg(target_os = "linux")]
use crate::device::uinput::UInputDev;

use anyhow::Result;

#[derive(Debug, Default)]
pub enum Device {
    /// Dummy device, does nothing.
    #[default]
    Dummy,
    /// Presents a virtual device using Linux's uinput.
    #[cfg(target_os = "linux")]
    UInput(UInputDev),
}

impl Device {
    pub fn set_wheel(&mut self, angle: f32) {
        match self {
            Device::Dummy => (),
            #[cfg(target_os = "linux")]
            Device::UInput(uinput_dev) => uinput_dev.set_wheel(angle),
        }
    }

    pub fn set_horn(&mut self, honking: bool) {
        match self {
            Device::Dummy => (),
            #[cfg(target_os = "linux")]
            Device::UInput(uinput_dev) => uinput_dev.set_horn(honking),
        }
    }

    pub fn apply(&mut self) -> Result<()> {
        match self {
            Device::Dummy => Ok(()),
            #[cfg(target_os = "linux")]
            Device::UInput(uinput_dev) => uinput_dev.apply(),
        }
    }
}