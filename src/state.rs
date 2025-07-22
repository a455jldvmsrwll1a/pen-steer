use crate::{config::Config, device::Device, pen::Pen, source::Source, wheel::Wheel};

#[derive(Debug)]
pub struct State {
    pub wheel: Wheel,
    pub pen: Option<Pen>,
    pub source: Source,
    pub device: Device,
    pub config: Config,
    pub outdated: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            wheel: Wheel::default(),
            pen: None,
            source: Source::Dummy,
            device: Device::Dummy,
            config: Config::default(),
            outdated: true,
        }
    }
}
