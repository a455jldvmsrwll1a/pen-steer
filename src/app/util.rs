/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::env::args;
use flume::Sender;
use log::{LevelFilter, error, info};

pub fn init_logging() {
    env_logger::Builder::default()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .filter_module("eframe", LevelFilter::Warn)
        .filter_module("calloop", LevelFilter::Warn)
        .filter_module("zbus", LevelFilter::Warn)
        .init();
}

pub fn set_handler(quit_tx: Sender<()>) {
    if let Err(err) = ctrlc::set_handler(move || {
        info!("Got stop request.");

        if let Err(err) = quit_tx.send(()) {
            error!("Could not send quit message: {err}");
        }
    }) {
        error!("Could not set signal handler: {err}");
    }
}

pub fn is_headless_requested() -> bool {
    args().any(|arg| arg.trim().eq_ignore_ascii_case("--headless"))
}
