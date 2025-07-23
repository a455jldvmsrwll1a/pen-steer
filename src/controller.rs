use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};

use crate::{
    config,
    device::{Device, uinput::UInputDev},
    source::{Source, net::NetSource},
    state::State,
    timer::Timer,
};

pub fn controller(state: Arc<Mutex<State>>) -> ! {
    let mut timer = Timer::new(state.lock().unwrap().config.update_frequency);

    loop {
        if let Err(err) = update(&mut state.lock().unwrap()) {
            eprintln!("Controller error: {err}");
        }

        timer.wait();
    }
}

pub fn update(state: &mut State) -> Result<()> {
    if state.outdated && state.device.is_none() {
        initialise_io(state)?;
    }

    if let Some(ref pen) = state.source.get() {
        state.pen = Some(pen.clone());

        if let Some(ctx) = &state.gui_context {
            ctx.request_repaint();
        }
    }

    state.wheel.update(
        state.device.as_mut(),
        &state.config,
        state.pen_override.clone().or_else(|| state.pen.clone()),
        1.0 / state.config.update_frequency as f32,
    );

    if let Some(device) = &mut state.device {
        device.apply().context("error applying device")?;
        device.handle_events();
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
        config::Source::Wintab => todo!(),
    };

    state.device = Some(match state.config.device {
        config::Device::None => Device::Dummy,
        config::Device::UInput => Device::UInput(
            UInputDev::new(&state.config).context("Could not set up uinput device!")?,
        ),
    });

    Ok(())
}
