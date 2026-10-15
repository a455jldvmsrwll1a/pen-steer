/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use anyhow::{Result, bail};
use arc_swap::ArcSwap;
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use log::{error, info, warn};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::core::config::{Profile, TickMode};
use crate::core::control::{self, Control};
use crate::core::input::{InputBackendEvent, InputBackendHandle};
use crate::core::math;
use crate::core::output::{FFbState, OutputBackendHandle, OutputEvent, OutputSample};
use crate::core::pen::Pen;
use crate::core::timer::Timer;
use crate::core::transform::Transformer;

#[derive(Debug)]
pub enum Command {
    WakeUp,
    ResetControlState,
    StartInput,
    StopInput,
    StartOutput,
    StopOutput,
}

pub enum Event {
    Error(anyhow::Error),
    InputStarted,
    OutputStarted,
    InputStopped,
    OutputStopped,
}

#[derive(Default, Clone)]
pub struct Snapshot {
    pub pen: Pen,
    pub grip_strength: f32,
    pub ffb_state: FFbState,
    pub output_state: OutputSample,
}

pub fn controller(
    config: Arc<ArcSwap<Profile>>,
    cmd_rx: Receiver<Command>,
    quit_rx: Receiver<()>,
    mut event_tx: Sender<Event>,
    mut snapshot_tx: triple_buffer::Input<Snapshot>,
) {
    info!("Controller starting.");

    let mut state = State::new(config);
    let quit = Rc::new(AtomicBool::new(false));
    let mut current_ffb_state = FFbState::Unavailable;

    'main: while !quit.load(Ordering::Acquire) {
        // Setup for wait.
        let mut selector = crossbeam_channel::Select::new();
        selector.recv(&quit_rx);
        selector.recv(&cmd_rx);

        if let Some(input) = &state.input {
            selector.recv(input.rx());
        }

        if let Some(output) = &state.output {
            selector.recv(output.event_rx());
            selector.recv(output.notification_rx());
        }

        if state.control.can_sleep() {
            selector.ready();
            state.timer.reset_next_tick();
        } else {
            let _ = selector.ready_timeout(Duration::ZERO);
        }

        // Process any new data after waiting.

        match quit_rx.try_recv() {
            Ok(..) | Err(TryRecvError::Disconnected) => {
                quit.store(true, Ordering::Release);

                break 'main;
            }
            Err(TryRecvError::Empty) => {}
        }

        for command in cmd_rx.try_iter() {
            if let Err(err) = state.process_command(&mut event_tx, command) {
                error!("Error processing command: {err:?}");
                let _ = event_tx.try_send(Event::Error(err));
            }
        }

        // Check for queued events from input.
        if let Some(input) = &mut state.input {
            loop {
                match input.rx().try_recv() {
                    Ok(InputBackendEvent::Started) => {
                        // we don't care; no more of these events should be
                        // emitted anyway
                    }
                    Ok(InputBackendEvent::Error(error)) => {
                        error!("Input backend error occured: {error}");
                        let _ = event_tx.try_send(Event::Error(error));
                    }
                    Ok(InputBackendEvent::Sample(input_sample)) => {
                        state.transformer.update(input_sample);
                    }
                    Err(TryRecvError::Disconnected) => {
                        state.input = None;
                        let _ = event_tx.try_send(Event::InputStopped);
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                }
            }
        }

        // Check for FFB state and queued events from output.
        if let Some(output) = &mut state.output {
            current_ffb_state = output.read_ffb_state_and_clear_notification().clone();

            loop {
                match output.event_rx().try_recv() {
                    Ok(OutputEvent::Started) => {
                        // we don't care; no more of these events should be
                        // emitted anyway
                    }
                    Ok(OutputEvent::Error(error)) => {
                        error!("Output backend error occured: {error}");
                        let _ = event_tx.try_send(Event::Error(error));
                    }
                    Err(TryRecvError::Disconnected) => {
                        state.output = None;
                        let _ = event_tx.try_send(Event::OutputStopped);
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                }
            }
        }

        let current_pen = state.transformer.get_pen();

        state.control.update(
            &state.config.load(),
            1.0 / 400.0,
            current_pen,
            &current_ffb_state,
            &mut state.output_state,
        );

        if let Some(output) = &mut state.output
            && state.output_state != state.prev_output_state
        {
            state.prev_output_state = state.output_state.clone();
            if let Err(err) = output.update_output(state.output_state.clone()) {
                error!("Error updating output state: {err:?}");
                let _ = event_tx.try_send(Event::Error(err));

                state.output = None;
                let _ = event_tx.try_send(Event::OutputStopped);
            }
        }

        let profile = &state.config.load().steering_wheel_control;
        let mut snapshot = snapshot_tx.input_buffer_publisher();
        snapshot.ffb_state = current_ffb_state.clone();
        snapshot.output_state = state.output_state.clone();
        snapshot.pen = current_pen;
        snapshot.grip_strength = math::remap(
            current_pen.pressure,
            profile.pressure_min / 100.0,
            profile.pressure_max / 100.0,
            0.0,
            1.0,
        )
        .clamp(0.0, 1.0);

        drop(snapshot);

        state.wait_for_next_tick();
    }

    info!("Controller stopping!");
}

struct State {
    config: Arc<ArcSwap<Profile>>,
    timer: Timer,
    control: Box<dyn Control>,
    input: Option<InputBackendHandle>,
    output: Option<OutputBackendHandle>,
    output_state: OutputSample,
    prev_output_state: OutputSample,
    transformer: Transformer,
}

impl State {
    fn new(config: Arc<ArcSwap<Profile>>) -> Self {
        Self {
            transformer: Transformer::new(&config.load_full().mapping),
            control: control::create_control(&config.load()),
            input: None,
            output: None,
            timer: Timer::with_frequency(400.0),
            output_state: OutputSample::default(),
            prev_output_state: OutputSample::default(),
            config,
        }
    }

    fn process_command(&mut self, event_tx: &mut Sender<Event>, command: Command) -> Result<()> {
        match command {
            Command::WakeUp => { /* ignore */ }
            Command::ResetControlState => {
                info!("Resetting control state...");

                self.control = control::create_control(&self.config.load());
            }
            Command::StartInput => {
                if self.input.is_some() {
                    warn!("Received StartInput command but input is already up!");

                    return Ok(());
                }

                info!("Starting input...");

                self.input = match InputBackendHandle::create(&self.config.load()) {
                    Ok(ibh) => Some(ibh),
                    Err(err) => bail!(err),
                };

                let _ = event_tx.try_send(Event::InputStarted);
            }
            Command::StopInput => {
                info!("Stopping input...");

                self.input = None;
                let _ = event_tx.try_send(Event::InputStopped);
            }
            Command::StartOutput => {
                if self.output.is_some() {
                    warn!("Received StartOutput command but output is already up!");

                    return Ok(());
                }

                info!("Starting output...");

                self.output = match OutputBackendHandle::create(&self.config.load()) {
                    Ok(obh) => Some(obh),
                    Err(err) => bail!(err),
                };

                let _ = event_tx.try_send(Event::OutputStarted);
            }
            Command::StopOutput => {
                info!("Stopping output...");

                self.output = None;
                let _ = event_tx.try_send(Event::OutputStopped);
            }
        }

        Ok(())
    }

    fn wait_for_next_tick(&mut self) {
        self.timer.wait_for_next_tick();
    }
}
