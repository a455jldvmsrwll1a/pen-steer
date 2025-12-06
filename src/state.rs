use anyhow::anyhow;
use log::{debug, warn};

use crate::{
    config::Config,
    device::Device,
    pen::Pen,
    save::{compile_parse_errors, load_file},
    save_path::save_path,
    source::Source,
    wheel::Wheel,
};

pub struct State {
    pub wheel: Wheel,
    pub pen: Option<Pen>,
    pub pen_override: Option<Pen>,
    pub source: Option<Box<dyn Source>>,
    pub device: Option<Box<dyn Device>>,
    pub config: Config,
    pub last_error: Option<anyhow::Error>,
    pub reset_source: bool,
    pub reset_device: bool,
}

impl State {
    pub fn create() -> Self {
        let mut state = Self::default();

        let path = save_path();
        debug!("Loading config at: {}", path.display());
        match load_file(&mut state.config, &path) {
            Ok(parse_errors) => {
                if !parse_errors.is_empty() {
                    state.last_error = Some(anyhow!(compile_parse_errors(parse_errors)));
                }
            }
            Err(load_err) => {
                // Do not show error if it just does not exist.
                let mut escalate_error = true;
                if let Some(err) = load_err.downcast_ref::<std::io::Error>() {
                    if let std::io::ErrorKind::NotFound = err.kind() {
                        escalate_error = false;
                    }
                }
                
                if escalate_error {
                    state.last_error = Some(load_err.context("Could not load configuration file."))
                } else {
                    warn!("Did not load a configuration file:\n{load_err:?}");
                }
            }
        }

        state
    }
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
            last_error: None,
            reset_source: true,
            reset_device: true,
        }
    }
}
