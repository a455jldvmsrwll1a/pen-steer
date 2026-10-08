/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use anyhow::{Result, bail};
use arc_swap::ArcSwap;
use flume::{Receiver, TryRecvError};
use log::{error, info, warn};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::config::Profile;
use crate::core::control::steering_wheel::SteeringWheel;
use crate::core::control::{self, Control};
use crate::core::input::{InputBackendEvent, InputBackendHandle, InputSample};
use crate::core::output::{self, OutputBackend};
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

    /*

    TODO: blocking mode loop is pretty much done ig
    TODO: NEXT: do the polling (timer) version, and then somehow mash them together

     */

    'main: while !quit.load(Ordering::Acquire) {
        let mut command = None;
        let mut sample = None;

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

                        sample = Some(input_sample);
                    }
                }
            }

            if input.rx().is_disconnected() {
                state.input = None;
            }
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

        let mut current_pen = None;
        if let Some(_sample) = &sample {
            let pen = state.transformer.get_pen();
            current_pen = Some(pen);
        }

        if let Some(command) = command.take() {
            if let Err(err) = state.process_command(command) {
                error!("Error processing command: {err}");
                let _ = event_tx.push(Event::Error(err));
            }
        }

        state.control.update(
            &state.config.load(),
            3.0 / 400.0,
            current_pen,
            state.output.as_deref_mut(),
        );

        if let Err(err) = state.output.as_deref_mut().apply() {
            error!("Error during output apply(): {err}");
            let _ = event_tx.push(Event::Error(err));

            state.output = None;
        }

        state.timer.wait();
    }

    info!("Controller stopping!");
}

struct State {
    config: Arc<ArcSwap<Profile>>,
    timer: Timer,
    control: Box<dyn Control>,
    input: Option<InputBackendHandle>,
    output: Option<Box<dyn OutputBackend>>,
    transformer: Transformer,
}

impl State {
    fn new(config: Arc<ArcSwap<Profile>>) -> Self {
        Self {
            transformer: Transformer::new(&config.load_full().mapping),
            control: control::create_control(&config.load()),
            input: None,
            output: None,
            timer: Timer::new(400.0 / 3.0),
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

                if self.input.is_none() {
                    return Ok(());
                }

                self.input = None;
            }
            Command::StartOutput => {
                info!("Starting output...");

                self.output = match output::create_output(&self.config.load()) {
                    Ok(out) => Some(out),
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
        self.timer.wait();
    }
}

struct TickEvents {
    command: Option<Command>,
    sample: Option<InputSample>,
    should_stop_input: bool,
}

// impl TickEvents {
//     fn wait_for_events()
// }
