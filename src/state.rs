use crate::{config::Config, device::Device, source::Source, wheel::Wheel};

#[derive(Debug, Default)]
pub struct State {
    pub wheel: Wheel,
    pub source: Source,
    pub device: Device,
    pub config: Config,
    pub outdated: bool,
}
