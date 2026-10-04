use std::sync::{Arc, atomic::AtomicBool};

use anyhow::{Result, bail};
use arc_swap::ArcSwap;
use log::{info, warn};
use triple_buffer::triple_buffer;

use crate::{
    config::Config,
    controller::{self, Snapshot},
    save::{compile_parse_errors, load_file},
    save_path::save_path,
    util,
};

pub fn run_headless() -> Result<()> {
    let quit_flag = Arc::new(AtomicBool::new(false));
    util::set_handler(quit_flag.clone());

    let save_path = save_path();

    info!("Loading configuration file: {}", save_path.display());
    let mut config = Config::default();
    match load_file(&mut config, &save_path) {
        Ok(parse_errors) => {
            if !parse_errors.is_empty() {
                bail!(
                    "Encountered parsing errors in configuration file:\n{}",
                    compile_parse_errors(parse_errors)
                );
            }
        }
        Err(load_err) => {
            if let Some(err) = load_err.downcast_ref::<std::io::Error>()
                && let std::io::ErrorKind::NotFound = err.kind()
            {
                warn!("No configuration file present. Using defaults...");
            } else {
                bail!(load_err.context("Could not load configuration file."));
            }
        }
    }
    info!("Configuration loaded.");

    let (cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1);
    let (event_tx, event_rx) = rtrb::RingBuffer::new(1);
    let (snapshot_tx, snapshot_rx) = triple_buffer(&Snapshot::default());

    drop((cmd_tx, event_rx, snapshot_rx));
    controller::controller(
        Arc::new(ArcSwap::new(Arc::new(config))),
        cmd_rx,
        event_tx,
        snapshot_tx,
        quit_flag,
    );

    info!("Bye.");

    Ok(())
}
