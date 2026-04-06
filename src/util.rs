use std::{
    env::args,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use log::{LevelFilter, error};

pub fn init_logging() {
    env_logger::Builder::default()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .filter_module("eframe", LevelFilter::Warn)
        .filter_module("calloop", LevelFilter::Warn)
        .init();
}

pub fn set_handler(quit_flag: Arc<AtomicBool>) {
    if let Err(err) = ctrlc::set_handler(move || {
        quit_flag.store(true, Ordering::Release);
    }) {
        error!("Could not set signal handler: {err}");
    }
}

pub fn is_headless_requested() -> bool {
    args().any(|arg| arg.trim().eq_ignore_ascii_case("--headless"))
}
