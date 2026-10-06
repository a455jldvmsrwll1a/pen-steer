/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use anyhow::{Result, bail};
use arc_swap::ArcSwap;
use flume::Receiver;
use log::{error, info, warn};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::config::Profile;
use crate::core::input::{InputBackendEvent, InputBackendHandle};
use crate::core::timer::Timer;
use crate::core::transform::{Pen, Transformer};

#[derive(Debug)]
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
    mut cmd_rx: Receiver<Command>,
    mut quit_rx: Receiver<()>,
    mut event_tx: rtrb::Producer<Event>,
    mut snapshot_tx: triple_buffer::Input<Snapshot>,
) {
    info!("Controller starting.");

    let mut state = State::new(config);
    let quit = Rc::new(AtomicBool::new(false));

    while !quit.load(Ordering::Acquire) {
        let mut command = None;
        let mut should_stop_input = false;
        let mut sample = None;

        let mut selector = flume::Selector::new();

        selector = selector.recv(&quit_rx, |_| {
            quit.store(true, Ordering::Release);
        });

        selector = selector.recv(&cmd_rx, |maybe_cmd| {
            let Ok(cmd) = maybe_cmd else {
                quit.store(true, Ordering::Release);

                return;
            };

            command = Some(cmd);
        });

        if let Some(input) = &mut state.input {
            selector = selector.recv(input.rx(), |maybe_event| {
                let Ok(event) = maybe_event else {
                    should_stop_input = true;

                    return;
                };

                match event {
                    InputBackendEvent::Started => {}
                    InputBackendEvent::Error(error) => {
                        error!("Input backend error occured: {error}");
                        let _ = event_tx.push(Event::Error(error));
                    }
                    InputBackendEvent::Sample(input_sample) => {
                        sample = Some(input_sample);
                    }
                }
            });
        }

        selector.wait();

        if let Some(sample) = &sample {
            state.transformer.update(*sample);
            let pen = state.transformer.get_pen();

            eprintln!(
                "btn: {:#08b}, press: {:5.1}%, aspect: {:.3}, x: {:4.2}, y: {:4.2}, X: {:5.2}, Y: {:5.2}, X': {:5.2}, Y': {:5.2}",
                sample.buttons,
                sample.pressure * 100.0,
                sample.aspect_ratio,
                sample.coordinate.x,
                sample.coordinate.y,
                pen.position.x,
                pen.position.y,
                pen.velocity.x,
                pen.velocity.y,
            );
        }

        if let Some(command) = command.take() {
            if let Err(err) = state.process_command(command) {
                error!("Error processing command: {err}");
                let _ = event_tx.push(Event::Error(err));
            }
        }
    }

    info!("Controller stopping!");
}

struct State {
    config: Arc<ArcSwap<Profile>>,
    timer: Timer,
    input: Option<InputBackendHandle>,
    transformer: Transformer,
}

impl State {
    pub fn new(config: Arc<ArcSwap<Profile>>) -> Self {
        Self {
            transformer: Transformer::new(&config.load_full().mapping),
            config,
            timer: Timer::new(60),
            input: None,
        }
    }

    fn process_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::ResetControlState => {
                warn!("TODO: Command::ResetControlState");
            }
            Command::StartInput => {
                info!("Starting input...");

                self.input = None;
                self.input = match InputBackendHandle::create(&self.config.load()) {
                    Ok(ibh) => Some(ibh),
                    Err(err) => bail!(err),
                };
            }
            Command::StopInput => {
                info!("Stopping input...");

                if self.input.is_none() {
                    return Ok(());
                }

                self.input = None;
            }
            Command::StartOutput => {
                warn!("TODO: Command::StartOutput");
            }
            Command::StopOutput => {
                warn!("TODO: Command::StopOutput");
            }
        }

        Ok(())
    }

    fn wait_for_next_tick(&mut self) {
        self.timer.wait();
    }
}
