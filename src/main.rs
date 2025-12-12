#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod controller;
mod device;
mod gui;
mod mapping;
mod math;
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
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
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

    let state = Arc::new(Mutex::new(State::create()));
    let quit_flag = Arc::new(AtomicBool::new(false));

    set_handler(quit_flag.clone());

    let cli_mode = args().any(|arg| arg.trim() == "--headless");
    if cli_mode {
        controller::controller(state, quit_flag);
        return Ok(());
    }

    let state_clone = state.clone();
    let quit_flag_clone = quit_flag.clone();
    let thread = std::thread::spawn(move || controller::controller(state_clone, quit_flag_clone));

    if let Err(err) = gui::gui(state, quit_flag.clone()) {
        bail!("GUI error: {err}");
    }

    quit_flag.store(true, Ordering::Release);
    let _ = thread.join();

    Ok(())
}

fn set_handler(quit_flag: Arc<AtomicBool>) {
    if let Err(err) = ctrlc::set_handler(move || {
        quit_flag.store(true, Ordering::Release);
    }) {
        error!("Could not set signal handler: {err}");
    }
}

fn init_logging() {
    env_logger::Builder::default()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .filter_module("eframe", LevelFilter::Warn)
        .filter_module("calloop", LevelFilter::Warn)
        .init();
}
