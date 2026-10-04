pub mod evdev;
pub mod net;

use crate::{config, input::net::NetworkInputBackend, pen::Pen};

#[cfg(target_os = "linux")]
use crate::input::evdev::EvdevInputBackend;

use anyhow::Result;

/// Backend for the input source.
///
pub trait InputBackend: Send + Sync {
    /// Get the current pen state, if any.
    /// 
    fn get(&mut self) -> Option<Pen>;
}

pub struct DummyInputBackend;

impl InputBackend for DummyInputBackend {
    fn get(&mut self) -> Option<Pen> {
        None
    }
}

pub fn create_source(config: &config::Config) -> Result<Box<dyn InputBackend>> {
    Ok(match config.source {
        config::Source::None => Box::new(DummyInputBackend),
        config::Source::Net => Box::new(NetworkInputBackend::new(&config.net_sock_addr)?),
        #[cfg(target_os = "windows")]
        config::Source::Wintab => Box::new(DummyInputBackend),
        #[cfg(target_os = "linux")]
        config::Source::Evdev => Box::new(EvdevInputBackend::new(config.preferred_tablet.as_deref())?),
    })
}
