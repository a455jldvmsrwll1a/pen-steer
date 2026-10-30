/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Instant,
};

use anyhow::{Result, anyhow, bail};
use crossbeam_channel::{Receiver, Sender, TrySendError};
use log::{debug, error, info};
use mio::{Events, Poll, Token, Waker};

use crate::core::config::Profile;

mod uinput;
mod vigem;

#[derive(Debug)]
pub enum OutputEvent {
    Started,
    Error(anyhow::Error),
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct OutputSample {
    /// `[ -1.0, 1.0 ]`
    pub wheel: f32,
    /// `[ 0.0, 1.0 ]`
    pub accelerator: f32,
    /// `[ 0.0, 1.0 ]`
    pub brake: f32,
    pub horn: bool,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum FFbState {
    #[default]
    Unavailable,
    Idle {
        timestamp: Instant,
    },
    Active {
        timestamp: Instant,
        /// `[ -1.0, 1.0 ]`
        constant_force: f32,
    },
}

#[derive(Debug)]
pub struct OutputBackendHandle {
    thread: Option<JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
    waker: Waker,
    output_tx: triple_buffer::Input<OutputSample>,
    feedback_rx: triple_buffer::Output<FFbState>,
    event_rx: Option<Receiver<OutputEvent>>,
    notification_rx: Option<Receiver<()>>,
}

impl OutputBackendHandle {
    const WAKE_TOKEN: Token = Token(4_12);

    pub fn event_rx(&self) -> &Receiver<OutputEvent> {
        self.event_rx
            .as_ref()
            .expect("should always be Some() up until destruction")
    }

    pub fn notification_rx(&self) -> &Receiver<()> {
        self.notification_rx
            .as_ref()
            .expect("should always be Some() up until destruction")
    }

    pub fn read_ffb_state_and_clear_notification(&mut self) -> &FFbState {
        while let Ok(()) = self.notification_rx().try_recv() {}

        self.feedback_rx.read()
    }

    pub fn update_output(&mut self, state: OutputSample) -> Result<()> {
        self.output_tx.write(state);
        self.waker.wake()?;

        Ok(())
    }

    pub fn create(profile: &Profile) -> Result<Self> {
        let conf = profile.clone();

        let waker_poll = Poll::new()?;
        let waker = Waker::new(waker_poll.registry(), Self::WAKE_TOKEN)?;

        let stop_flag = Arc::new(AtomicBool::new(false));
        let (event_tx, event_rx) = crossbeam_channel::bounded(8192);
        let (notification_tx, notification_rx) = crossbeam_channel::bounded(1);
        let (feedback_tx, feedback_rx) = triple_buffer::triple_buffer(&FFbState::Unavailable);
        let (output_tx, output_rx) = triple_buffer::triple_buffer(&OutputSample::default());

        let obs = OutputBackendStuffs {
            event_tx: event_tx.clone(),
            notification_tx,
            feedback_tx,
            output_rx,
            stop_flag: stop_flag.clone(),
            waker_poll,
        };

        let event_tx_2 = event_tx.clone();
        let thread = std::thread::spawn(move || {
            info!("Starting {:?} output...", conf.active_output);

            let result = match conf.active_output {
                super::config::OutputBackend::Null => null_thread_loop(obs),
                #[cfg(target_os = "linux")]
                super::config::OutputBackend::LinuxUinput => {
                    uinput::uinput_thread_loop(&conf.linux_uinput_output, obs)
                }
                #[cfg(not(target_os = "linux"))]
                super::config::OutputBackend::LinuxUinput => {
                    Err(anyhow!("LinuxUinput supports Linux only."))
                }
                #[cfg(target_os = "linux")]
                super::config::OutputBackend::LinuxUsbPeripheral => {
                    Err(anyhow!("LinuxUsbPeripheral not yet implemented."))
                }
                #[cfg(not(target_os = "linux"))]
                super::config::OutputBackend::LinuxUsbPeripheral => {
                    Err(anyhow!("LinuxUsbPeripheral supports Linux only."))
                }
                #[cfg(target_os = "windows")]
                super::config::OutputBackend::WindowsVigemBus => {
                    vigem::thread_loop(&conf.windows_vigembus_output, obs)
                }
                #[cfg(not(target_os = "windows"))]
                super::config::OutputBackend::WindowsVigemBus => {
                    Err(anyhow!("WindowsVigemBus supports Windows only."))
                }
                super::config::OutputBackend::UsbOverIpServer => {
                    Err(anyhow!("UsbOverIpServer not yet implemented."))
                }
            };

            if let Err(err) = result {
                error!(
                    "Output {:?} thread terminated with error: {err:?}",
                    conf.active_output
                );
                let _ = event_tx_2.send(OutputEvent::Error(err));
            } else {
                info!("Output {:?} terminated normally.", conf.active_output);
            }
        });

        let first_event = event_rx.recv().expect("could not wait for success signal");
        if !matches!(first_event, OutputEvent::Started) {
            stop_flag.store(true, Ordering::Release);
            let _ = waker.wake();

            bail!("Did not receive success signal from output thread.");
        }

        Ok(Self {
            thread: Some(thread),
            stop_flag,
            waker,
            event_rx: Some(event_rx),
            output_tx,
            feedback_rx,
            notification_rx: Some(notification_rx),
        })
    }
}

impl Drop for OutputBackendHandle {
    fn drop(&mut self) {
        debug!("Dropping OutputBackendHandle.");

        self.event_rx.take();
        self.stop_flag.store(true, Ordering::Release);
        let _ = self.waker.wake();

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct OutputBackendStuffs {
    event_tx: Sender<OutputEvent>,
    notification_tx: Sender<()>,
    feedback_tx: triple_buffer::Input<FFbState>,
    output_rx: triple_buffer::Output<OutputSample>,
    stop_flag: Arc<AtomicBool>,
    waker_poll: mio::Poll,
}

impl OutputBackendStuffs {
    fn send_event(&self, event: OutputEvent) -> Result<()> {
        self.event_tx.send(event)?;

        Ok(())
    }

    fn update_feedback(&mut self, ffb: FFbState) -> Result<()> {
        let outgoing_state = self.feedback_tx.input_buffer();
        if ffb == *outgoing_state {
            return Ok(());
        }

        self.feedback_tx.write(ffb);
        if let Err(TrySendError::Disconnected(..)) = self.notification_tx.try_send(()) {
            bail!("notification channel was disconnected");
        }

        Ok(())
    }

    fn should_stop(&self) -> bool {
        self.stop_flag.load(Ordering::Acquire)
    }
}

fn null_thread_loop(mut obs: OutputBackendStuffs) -> Result<()> {
    let mut events = Events::with_capacity(0);

    obs.send_event(OutputEvent::Started)?;

    while !obs.should_stop() {
        obs.waker_poll.poll(&mut events, None)?;
    }

    Ok(())
}
