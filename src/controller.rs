use std::{sync::{Arc, Mutex}, thread, time::Duration};
use anyhow::Result;

use crate::{config, source::{net::NetSource, Source}, state::State, timer::Timer};

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

    Ok(())
}

pub fn initialise_io(state: &mut State) -> Result<()> {
    state.source = match state.config.source {
        config::Source::None => Source::Dummy,
        config::Source::Net => Source::Net(NetSource::new(&state.config.net_sock_addr)?),
        config::Source::Wintab => todo!(),
    };

    state.outdated = false;

    Ok(())
}
