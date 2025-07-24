use crate::{config::Config, device::Device, pen::Pen, source::Source, wheel::Wheel};

pub struct State {
    pub wheel: Wheel,
    pub pen: Option<Pen>,
    pub pen_override: Option<Pen>,
    pub source: Option<Box<dyn Source>>,
    pub device: Option<Box<dyn Device>>,
    pub config: Config,
    pub gui_context: Option<eframe::egui::Context>,
    pub reset_pending: bool,
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
            gui_context: None,
            reset_pending: true,
        }
    }
}
