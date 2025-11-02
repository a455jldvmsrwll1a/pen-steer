#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod controller;
mod device;
mod gui;
mod mapping;
mod pen;
mod save;
mod save_path;
mod source;
mod state;
mod timer;
mod wheel;

use std::{
    env::args,
    fs::create_dir_all,
    sync::{Arc, Mutex},
};

use anyhow::{Result, bail};

use log::{LevelFilter, error, info};

use crate::{save_path::save_dir, state::State};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    init_logging();
    info!("pen-steer v{VERSION}");

    if let Err(err) = create_dir_all(save_dir()) {
        error!("Could not create configuration directory: {err}");
    }

    let cli_mode = args().any(|arg| arg.trim() == "--headless");

    if cli_mode {
        start_headless()
    } else {
        start_gui()
    }
}

fn start_gui() -> Result<()> {
    let state = Arc::new(Mutex::new(State::create()));

    let state_clone = state.clone();
    std::thread::spawn(move || controller::controller(state_clone));

    if let Err(err) = gui::gui(state.clone()) {
        bail!("GUI error: {err}");
    }

    Ok(())
}

fn start_headless() -> ! {
    controller::controller(Arc::new(Mutex::new(State::create())));
}

fn init_logging() {
    env_logger::Builder::default()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .filter_module("eframe", LevelFilter::Warn)
        .filter_module("calloop", LevelFilter::Warn)
        .init();
}
