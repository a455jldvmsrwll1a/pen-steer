#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod controller;
mod gui_old;
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

use std::{
    fs::create_dir_all,
    sync::{Arc, atomic::AtomicBool},
};

use anyhow::{Result, bail};

use log::{error, info};
use slint::ToSharedString;

use crate::save_path::save_dir;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_ID: &str = "ph.jldvmsrwll1a.pen-steer";

slint::include_modules!();

fn main() -> Result<()> {
    util::init_logging();
    info!("pen-steer v{VERSION}");

    let is_running = Arc::new(AtomicBool::default());

    let ui = AppWindow::new()?;
    slint::set_xdg_app_id(APP_ID)?;
    ui.set_app_version(VERSION.to_shared_string());

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

    let is_running_clone = is_running.clone();
    let ui_weak = ui.as_weak();
    ui.on_main_action_clicked(move || {
        is_running_clone.fetch_not(std::sync::atomic::Ordering::SeqCst);

        ui_weak
            .unwrap()
            .invoke_set_running_state(is_running_clone.load(std::sync::atomic::Ordering::SeqCst));
    });

    ui.run()?;

    return Ok(());

    if let Err(err) = create_dir_all(save_dir()) {
        error!("Could not create configuration directory: {err}");
    }

    if util::is_headless_requested() {
        return headless::run_headless();
    }

    if let Err(err) = gui_old::gui() {
        bail!("GUI error: {err}");
    }

    Ok(())
}
