use anyhow::{Context, Result};
use log::{debug, error, info};
use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::device::create_device;
use crate::source::create_source;
use crate::{state::State, timer::Timer};

pub fn controller(state: Arc<Mutex<State>>) -> ! {
    let mut update_frequency = state.lock().unwrap().config.update_frequency;
    info!("Using {update_frequency} Hz rate.");
    let mut timer = Timer::new(update_frequency);

    loop {
        if let Err(err) = update(&mut state.lock().unwrap()) {
            error!("Controller error: {err}");
        }

        let current_update_frequency = state.lock().unwrap().config.update_frequency;
        if current_update_frequency != update_frequency {
            update_frequency = current_update_frequency;
            timer = Timer::new(update_frequency);
            info!("Now updating at {update_frequency} Hz.");
        }

        timer.wait();
    }
}

pub fn update(state: &mut State) -> Result<()> {
    if state.reset_pending {
        initialise_io(state)?;
    }

    let mut needs_redraw = false;

    if let Some(Some(ref pen)) = state.source.as_mut().map(|s| s.get()) {
        state.pen = Some(pen.clone());
        needs_redraw = true;
    }

    needs_redraw |= state.wheel.update(
        state.device.as_mut(),
        &state.config,
        state.pen_override.clone().or_else(|| state.pen.clone()),
        1.0 / state.config.update_frequency as f32,
    );

    if let Some(device) = &mut state.device {
        device.apply().context("error applying device")?;
        device.handle_events();
    }

    if needs_redraw && let Some(ctx) = &state.gui_context {
        ctx.request_repaint();
    }

    Ok(())
}

fn initialise_io(state: &mut State) -> Result<()> {
    debug!("initialising I/O");

    state.pen = None;
    state.reset_pending = false;

    if let Err(e) = create_source_and_device(state) {
        error!("Failed to create source and device!");
        
        state.source = None;
        state.device = None;

        return Err(e);
    }

    Ok(())
}

fn create_source_and_device(state: &mut State) -> Result<()> {
    state.source = Some(create_source(&state.config)?);
    state.device = Some(create_device(&state.config)?);

    Ok(())
}