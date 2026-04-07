#[cfg(target_os = "linux")]
pub mod evdev;
pub mod net;

use crate::{config, pen::Pen, source::net::NetSource};

#[cfg(target_os = "linux")]
use crate::source::evdev::EvdevSource;

use anyhow::Result;

pub trait Source: Send + Sync {
    fn get(&mut self) -> Option<Pen>;
}

pub struct DummySource;

impl Source for DummySource {
    fn get(&mut self) -> Option<Pen> {
        None
    }
}

pub fn create_source(config: &config::Config) -> Result<Box<dyn Source>> {
    Ok(match config.source {
        config::Source::None => Box::new(DummySource),
        config::Source::Net => Box::new(NetSource::new(&config.net_sock_addr)?),
        #[cfg(target_os = "windows")]
        config::Source::Wintab => Box::new(DummySource),
        #[cfg(target_os = "linux")]
        config::Source::Evdev => Box::new(EvdevSource::new(config.preferred_tablet.as_deref())?),
    })
}
