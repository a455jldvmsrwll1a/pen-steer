use anyhow::Result;
use arc_swap::ArcSwap;
use log::{error, info, warn};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::config::Profile;
use crate::core::input::InputBackendHandle;
use crate::core::timer::Timer;
use crate::core::transform::Pen;

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
    pub pen: Option<Pen>,
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

    let mut state = State::new(config);

    while !quit_flag.load(Ordering::Acquire) {
        if let Ok(command) = cmd_rx.pop()
            && let Err(err) = state.process_command(command)
        {
            error!("Error processing command: {err}");
            let _ = event_tx.push(Event::Error(err));
        }

        state.wait_for_next_tick();
    }

    info!("Controller stopping!");
}

struct State {
    config: Arc<ArcSwap<Profile>>,
    timer: Timer,
    input: Option<InputBackendHandle>,
}

impl State {
    pub fn new(config: Arc<ArcSwap<Profile>>) -> Self {
        Self {
            config,
            timer: Timer::new(60),
            input: None,
        }
    }

    fn process_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::ResetControlState => {
                warn!("TODO: Command::ResetControlState");
            },
            Command::StartInput => {

            },
            Command::StopInput => {
                
            },
            Command::StartOutput => {
                warn!("TODO: Command::StartOutput");
            },
            Command::StopOutput => {
                warn!("TODO: Command::StopOutput");
            },
        }

        Ok(())
    }

    fn wait_for_next_tick(&mut self) {
        self.timer.wait();
    }
}