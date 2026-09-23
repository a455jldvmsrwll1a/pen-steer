#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod controller;
mod gui;
mod headless;
mod input;
mod mapping;
mod math;
mod output;
mod pen;
mod save;
mod save_path;
mod timer;
mod util;
mod wheel;

use std::fs::create_dir_all;

use anyhow::{Result, bail};

use log::{error, info};
use slint::ToSharedString;

use crate::save_path::save_dir;

const VERSION: &str = env!("CARGO_PKG_VERSION");

slint::include_modules!();

fn main() -> Result<()> {
    util::init_logging();
    info!("pen-steer v{VERSION}");

    let ui = AppWindow::new()?;
    ui.set_app_version(VERSION.to_shared_string());

    let ui_handle = ui.as_weak();
    ui.on_request_increase_value(move || {
        let ui = ui_handle.unwrap();
        ui.set_counter(ui.get_counter() + 1);
    });

    ui.on_show_about_app(move || {
        if let Ok(window) = AboutWindow::new() {
            let _ = window.run();
        }
    });

    ui.on_show_about_slint(move || {
        if let Ok(window) = SlintWindow::new() {
            let _ = window.run();
        }
    });

    ui.run()?;

    return Ok(());

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
