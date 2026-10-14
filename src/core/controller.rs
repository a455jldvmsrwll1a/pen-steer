/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use anyhow::{Result, bail};
use arc_swap::ArcSwap;
use flume::{Receiver, TryRecvError};
use log::{error, info};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::config::Profile;
use crate::core::control::{self, Control};
use crate::core::input::{InputBackendEvent, InputBackendHandle};
use crate::core::math;
use crate::core::output::{FFbState, OutputBackendHandle, OutputSample};
use crate::core::pen::Pen;
use crate::core::timer::Timer;
use crate::core::transform::Transformer;

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
    pub pen: Pen,
    pub grip_strength: f32,
    pub ffb_state: FFbState,
    pub output_state: OutputSample,
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
    let mut current_ffb_state = FFbState::Unavailable;

    'main: while !quit.load(Ordering::Acquire) {
        let mut command = None;

        match quit_rx.try_recv() {
            Ok(..) | Err(TryRecvError::Disconnected) => {
                quit.store(true, Ordering::Release);

                break 'main;
            }
            Err(TryRecvError::Empty) => {}
        }

        match cmd_rx.try_recv() {
            Ok(cmd) => command = Some(cmd),
            Err(TryRecvError::Disconnected) => {
                quit.store(true, Ordering::Release);

                break 'main;
            }
            Err(TryRecvError::Empty) => {}
        }

        if let Some(input) = &mut state.input {
            for event in input.rx().try_iter() {
                match event {
                    InputBackendEvent::Started => {}
                    InputBackendEvent::Error(error) => {
                        error!("Input backend error occured: {error}");
                        let _ = event_tx.push(Event::Error(error));
                    }
                    InputBackendEvent::Sample(input_sample) => {
                        state.transformer.update(input_sample);
                    }
                }
            }

            if input.rx().is_disconnected() {
                state.input = None;
            }
        }

        if let Some(output) = &mut state.output {
            current_ffb_state = output.read_feedback_state().clone();
        }

        // let mut selector = flume::Selector::new();

        // selector = selector.recv(&quit_rx, |_| {
        //     quit.store(true, Ordering::Release);
        // });

        // selector = selector.recv(&cmd_rx, |maybe_cmd| {
        //     let Ok(cmd) = maybe_cmd else {
        //         quit.store(true, Ordering::Release);

        //         return;
        //     };

        //     command = Some(cmd);
        // });

        // if let Some(input) = &mut state.input {
        //     selector = selector.recv(input.rx(), |maybe_event| {
        //         let Ok(event) = maybe_event else {
        //             should_stop_input = true;

        //             return;
        //         };

        //         match event {
        //             InputBackendEvent::Started => {}
        //             InputBackendEvent::Error(error) => {
        //                 error!("Input backend error occured: {error}");
        //                 let _ = event_tx.push(Event::Error(error));
        //             }
        //             InputBackendEvent::Sample(input_sample) => {
        //                 sample = Some(input_sample);
        //             }
        //         }
        //     });
        // }

        // selector.wait();

        let current_pen = state.transformer.get_pen();

        if let Some(command) = command.take() {
            if let Err(err) = state.process_command(command) {
                error!("Error processing command: {err:?}");
                let _ = event_tx.push(Event::Error(err));
            }
        }

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
                let _ = event_tx.push(Event::Error(err));

                state.output = None;
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

    fn process_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::ResetControlState => {
                info!("Resetting control state...");

                self.control = control::create_control(&self.config.load());
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

                self.input = None;
            }
            Command::StartOutput => {
                info!("Starting output...");

                self.output = match OutputBackendHandle::create(&self.config.load()) {
                    Ok(obh) => Some(obh),
                    Err(err) => bail!(err),
                };
            }
            Command::StopOutput => {
                info!("Stopping output...");

                self.output = None;
            }
        }

        Ok(())
    }

    fn wait_for_next_tick(&mut self) {
        self.timer.wait_for_next_tick();
    }
}
