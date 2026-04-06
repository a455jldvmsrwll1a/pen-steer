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
mod util;
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

use log::{error, info};

use crate::{save_path::save_dir, state::State};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    util::init_logging();
    info!("pen-steer v{VERSION}");

    if let Err(err) = create_dir_all(save_dir()) {
        error!("Could not create configuration directory: {err}");
    }

    let state = Arc::new(Mutex::new(State::create()));
    let quit_flag = Arc::new(AtomicBool::new(false));

    util::set_handler(quit_flag.clone());

    if util::is_headless_requested() {
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
