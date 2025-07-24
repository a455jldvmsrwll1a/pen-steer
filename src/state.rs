use crate::{config::Config, device::Device, pen::Pen, source::Source, wheel::Wheel};

pub struct State {
    pub wheel: Wheel,
    pub pen: Option<Pen>,
    pub pen_override: Option<Pen>,
    pub source: Option<Box<dyn Source>>,
    pub device: Option<Box<dyn Device>>,
    pub config: Config,
    pub gui_context: Option<eframe::egui::Context>,
    pub last_error: Option<anyhow::Error>,
    pub reset_source: bool,
    pub reset_device: bool,
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
            last_error: None,
            reset_source: true,
            reset_device: true,
        }
    }
}
