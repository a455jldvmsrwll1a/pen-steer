#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

mod app;

use anyhow::Result;
use log::info;

const APP_ID: &str = "ph.jldvmsrwll1a.pen-steer";

slint::include_modules!();

fn main() -> Result<()> {
    app::util::init_logging();
    info!("pen-steer v{}", pen_steer::PACKAGE_VERSION);

    app::start_app()?;

    info!("Bye.");

    Ok(())
}
