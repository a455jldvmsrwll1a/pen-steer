#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod device;
mod gui;
mod source;
mod state;
mod wheel;

use std::sync::{Arc, Mutex};

use anyhow::{Result, bail};

use crate::state::State;

fn main() -> Result<()> {
    let state = Arc::new(Mutex::new(State::default()));

    if let Err(err) = gui::gui(state.clone()) {
        bail!("GUI error: {err}");
    }

    Ok(())
}
