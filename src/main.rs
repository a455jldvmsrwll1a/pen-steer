#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

mod app;

use log::{error, info};

const APP_ID: &str = "ph.jldvmsrwll1a.pen-steer";

slint::include_modules!();

fn main() {
    app::util::init_logging();
    info!("Hello world! pen-steer v{}", pen_steer::PACKAGE_VERSION);

    if let Err(err) = app::start_app() {
        error!("An error occured: {err}");
    }

    info!("Bye.");
}
