use anyhow::Result;
use std::sync::{Arc, Mutex};

use crate::{
    config,
    source::{Source, net::NetSource},
    state::State,
    timer::Timer,
};

pub fn controller(state: Arc<Mutex<State>>) {
    let mut timer = Timer::new(state.lock().unwrap().config.update_frequency);

    loop {
        if let Err(err) = update(&mut state.lock().unwrap()) {
            eprintln!("Controller error: {err}");
        }

        timer.wait();
    }
}

pub fn update(state: &mut State) -> Result<()> {
    if state.outdated {
        initialise_io(state)?;
    }

    if let Some(ref pen) = state.source.get() {
        state.pen = Some(pen.clone());
    }

    state.wheel.update(
        &state.config,
        state.pen_override.clone().or_else(|| state.pen.clone()),
        1.0 / state.config.update_frequency as f32,
    );

    Ok(())
}

pub fn initialise_io(state: &mut State) -> Result<()> {
    state.source = match state.config.source {
        config::Source::None => Source::Dummy,
        config::Source::Net => Source::Net(NetSource::new(&state.config.net_sock_addr)?),
        config::Source::Wintab => todo!(),
    };

    state.pen = None;
    state.outdated = false;

    Ok(())
}
