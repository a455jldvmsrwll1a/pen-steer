use anyhow::Result;
use arc_swap::ArcSwap;
use log::info;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::Profile;
use crate::pen::{Pen, PostTransform};
use crate::timer::Timer;

pub enum Command {
    ResetControlState,
    StartInput,
    StopInput,
    StartOutput,
    StopOutput,
}

pub enum Event {
    Error(anyhow::Error),
}

#[derive(Default, Clone)]
pub struct Snapshot {
    pub pen: Option<Pen<PostTransform>>,
    pub ffb_value: Option<f32>,
}

pub fn controller(
    config: Arc<ArcSwap<Profile>>,
    mut cmd_rx: rtrb::Consumer<Command>,
    mut event_tx: rtrb::Producer<Event>,
    mut snapshot_tx: triple_buffer::Input<Snapshot>,
    quit_flag: Arc<AtomicBool>,
) {
    info!("Controller starting.");

    let mut timer = Timer::new(60);

    while !quit_flag.load(Ordering::Acquire) {
        timer.wait();
    }

    info!("Controller stopping!");
}
