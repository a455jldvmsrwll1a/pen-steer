#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod controller;
mod device;
mod gui;
mod pen;
mod source;
mod state;
mod timer;
mod wheel;

use std::sync::{Arc, Mutex};

use anyhow::{Result, bail};

use crate::state::State;

fn main() -> Result<()> {
    let cli_mode = std::env::args().any(|arg| arg.trim() == "--headless");

    if cli_mode {
        start_headless()
    } else {
        start_gui()
    }
}

fn start_gui() -> Result<()> {
    let state = Arc::new(Mutex::new(State::default()));

    let state_clone = state.clone();
    std::thread::spawn(move || controller::controller(state_clone));

    if let Err(err) = gui::gui(state.clone()) {
        bail!("GUI error: {err}");
    }

    Ok(())
}

fn start_headless() -> ! {
    controller::controller(Arc::new(Mutex::new(State::default())));
}
