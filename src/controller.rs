use anyhow::{Context, Result};
use log::{debug, error, info};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::Config;
use crate::device::{Device, create_device};
use crate::pen::Pen;
use crate::source::{Source, create_source};
use crate::timer::Timer;
use crate::wheel::Wheel;

pub enum Command {
    /// Set config and reset everything.
    Initialise { new_config: Config },
    /// Only set config.
    UpdateConfig { new_config: Config },
    /// Only set wheel.
    TeleportWheel { new_wheel: Wheel },
    /// Only set wheel.
    SetPenOverride { pen: Option<Pen> },
    /// Only reset source.
    ResetSource,
    /// Only reset device.
    ResetDevice,
}

pub enum Event {
    Error(anyhow::Error),
}

#[derive(Default, Clone)]
pub struct Snapshot {
    pub pen: Option<Pen>,
    pub wheel: Wheel,
    pub feedback: Option<f32>,
}

pub fn controller(
    initial_config: Config,
    mut cmd_rx: rtrb::Consumer<Command>,
    mut event_tx: rtrb::Producer<Event>,
    mut snapshot_tx: triple_buffer::Input<Snapshot>,
    quit_flag: Arc<AtomicBool>,
) {
    info!("Controller thread started.");

    let mut state = State {
        config: initial_config,
        wheel: Wheel::default(),
        pen: None,
        pen_override: None,
        source: None,
        device: None,
    };

    let mut ups = state.config.update_frequency;
    info!("Using {} Hz rate.", ups);
    let mut timer = Timer::new(ups);

    if let Err(err) = state.reset_source() {
        error!("Error initialising source: {err}");
        let _ = event_tx.push(Event::Error(err));
    }
    if let Err(err) = state.reset_device() {
        error!("Error initialising device: {err}");
        let _ = event_tx.push(Event::Error(err));
    }

    while !quit_flag.load(Ordering::Acquire) {
        if let Ok(command) = cmd_rx.pop() {
            if let Err(err) = state.process_command(command) {
                error!("Error processing command: {err}");
                let _ = event_tx.push(Event::Error(err));
            }
        }

        let current_update_frequency = state.config.update_frequency;
        if current_update_frequency != ups {
            ups = current_update_frequency;
            timer = Timer::new(ups);
            info!("Now updating at {ups} Hz.");
        }

        if let Err(err) = state.update().context("Error during controller tick.") {
            error!("Controller error: {err}");
            let _ = event_tx.push(Event::Error(err));
        }

        snapshot_tx.write(state.take_snapshot());

        timer.wait();
    }

    info!("Controller stopping!");
}

struct State {
    config: Config,
    wheel: Wheel,
    pen: Option<Pen>,
    pen_override: Option<Pen>,
    source: Option<Box<dyn Source>>,
    device: Option<Box<dyn Device>>,
}

impl State {
    fn take_snapshot(&self) -> Snapshot {
        Snapshot {
            pen: self.pen.or(self.pen_override),
            wheel: self.wheel.clone(),
            feedback: self.device.as_ref().map(|dev| dev.get_feedback()).flatten(),
        }
    }

    fn process_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::Initialise { new_config } => {
                self.config = new_config;
                self.reset_source()?;
                self.reset_device()?;
            }
            Command::UpdateConfig { new_config } => {
                self.config = new_config;
            }
            Command::TeleportWheel { new_wheel } => {
                self.wheel = new_wheel;
            }
            Command::SetPenOverride { pen } => {
                self.pen_override = pen;
            }
            Command::ResetSource => {
                self.reset_source()?;
            }
            Command::ResetDevice => {
                self.reset_device()?;
            }
        }

        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        if let Some(Some(raw_pen)) = self.source.as_mut().map(|s| s.get()) {
            let pen = self.config.mapping.pen(raw_pen);
            self.pen = Some(pen);
        }

        #[allow(clippy::cast_possible_truncation)]
        self.wheel.update(
            self.device.as_mut(),
            &self.config,
            self.pen_override.or(self.pen),
            1.0 / self.config.update_frequency as f32,
        );

        if let Some(device) = &mut self.device {
            device.apply().context("error applying device")?;
            device.handle_events();
        }

        Ok(())
    }

    fn reset_source(&mut self) -> Result<()> {
        debug!("resetting source.");

        self.pen = None;
        self.source = None;

        match create_source(&self.config) {
            Ok(source) => self.source = Some(source),
            Err(err) => {
                error!("Failed to create source!");
                return Err(err);
            }
        }

        Ok(())
    }

    fn reset_device(&mut self) -> Result<()> {
        debug!("resetting device.");

        self.pen = None;
        self.device = None;

        match create_device(&self.config).context("Could not create device.") {
            Ok(device) => self.device = Some(device),
            Err(err) => {
                error!("Failed to create device!");
                return Err(err);
            }
        }

        Ok(())
    }
}
