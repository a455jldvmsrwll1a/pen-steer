#[cfg(target_os = "linux")]
pub mod evdev;
pub mod net;

use crate::{pen::Pen, source::net::NetSource};

#[cfg(target_os = "linux")]
use crate::source::evdev::EvdevSource;

#[derive(Debug, Default)]
pub enum Source {
    /// Dummy source, does nothing.
    #[default]
    Dummy,
    /// Receive input events from external software via network.
    Net(NetSource),
    /// Reads input events from /dev/input/eventX.
    #[cfg(target_os = "linux")]
    Evdev(EvdevSource),
}

impl Source {
    pub fn get(&mut self) -> Option<Pen> {
        match self {
            Source::Dummy => None,
            Source::Net(net_source) => net_source.try_read(),
            #[cfg(target_os = "linux")]
            Self::Evdev(evdev_source) => evdev_source.try_read(),
        }
    }
}
