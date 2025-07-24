use crate::{config::Config, device::Device, pen::Pen, source::Source, wheel::Wheel};

pub struct State {
    pub wheel: Wheel,
    pub pen: Option<Pen>,
    pub pen_override: Option<Pen>,
    pub source: Option<Box<dyn Source>>,
    pub device: Option<Box<dyn Device>>,
    pub config: Config,
    pub outdated: bool,
    pub gui_context: Option<eframe::egui::Context>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            wheel: Wheel::default(),
            pen: None,
            pen_override: None,
            source: None,
            device: None,
            config: Config::default(),
            outdated: true,
            gui_context: None,
        }
    }
}
