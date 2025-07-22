#[cfg(target_os = "linux")]
pub mod uinput;

#[cfg(target_os = "linux")]
use crate::device::uinput::UInputDev;

#[derive(Debug, Default)]
pub enum Device {
    /// Dummy device, does nothing.
    #[default]
    Dummy,
    /// Presents a virtual device using Linux's uinput.
    #[cfg(target_os = "linux")]
    UInput(UInputDev),
}
