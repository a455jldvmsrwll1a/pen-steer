#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod controller;
mod device;
mod gui;
mod headless;
mod mapping;
mod math;
mod pen;
mod save;
mod save_path;
mod source;
mod timer;
mod util;
mod wheel;

use std::fs::create_dir_all;

use anyhow::{Result, bail};

use log::{error, info};

use crate::save_path::save_dir;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    util::init_logging();
    info!("pen-steer v{VERSION}");

    if let Err(err) = create_dir_all(save_dir()) {
        error!("Could not create configuration directory: {err}");
    }

    if util::is_headless_requested() {
        return headless::run_headless();
    }

    if let Err(err) = gui::gui() {
        bail!("GUI error: {err}");
    }

    Ok(())
}
