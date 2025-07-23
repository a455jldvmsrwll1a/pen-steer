use anyhow::{Context, Result};
use log::error;
use std::sync::{Arc, Mutex};

use crate::{
    config,
    device::Device,
    source::{Source, net::NetSource},
    state::State,
    timer::Timer,
};

#[cfg(target_os = "linux")]
use crate::device::uinput::UInputDev;
#[cfg(target_os = "linux")]
use crate::source::evdev::EvdevSource;

pub fn controller(state: Arc<Mutex<State>>) -> ! {
    let mut update_frequency = state.lock().unwrap().config.update_frequency;
    let mut timer = Timer::new(update_frequency);

    loop {
        if let Err(err) = update(&mut state.lock().unwrap()) {
            error!("Controller error: {err}");
        }

        let current_update_frequency = state.lock().unwrap().config.update_frequency;
        if current_update_frequency != update_frequency {
            update_frequency = current_update_frequency;
            timer = Timer::new(update_frequency);
        }

        timer.wait();
    }
}

pub fn update(state: &mut State) -> Result<()> {
    if state.outdated && state.device.is_none() {
        initialise_io(state)?;
    }

    let mut needs_redraw = false;

    if let Some(ref pen) = state.source.get() {
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

pub fn initialise_io(state: &mut State) -> Result<()> {
    state.pen = None;
    state.outdated = false;

    state.source = Source::Dummy;
    state.device = None;

    state.source = match state.config.source {
        config::Source::None => Source::Dummy,
        config::Source::Net => Source::Net(NetSource::new(&state.config.net_sock_addr)?),
        #[cfg(target_os = "windows")]
        config::Source::Wintab => Source::Dummy,
        #[cfg(target_os = "linux")]
        config::Source::Evdev => {
            Source::Evdev(EvdevSource::new(state.config.preferred_tablet.as_deref())?)
        }
    };

    state.device = Some(match state.config.device {
        config::Device::None => Device::Dummy,
        #[cfg(target_os = "linux")]
        config::Device::UInput => Device::UInput(
            UInputDev::new(&state.config).context("Could not set up uinput device!")?,
        ),
        #[cfg(target_os = "windows")]
        config::Device::VigemBus => Device::Dummy,
    });

    Ok(())
}
