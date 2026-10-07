/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

pub mod net;

use anyhow::{Result, anyhow, bail};
use flume::{Receiver, Sender};
use log::{debug, error, info};
use mio::{Events, Poll, Token, Waker};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Instant,
};

use crate::core::config::Profile;
use glam::Vec2;

#[derive(Debug)]
pub enum InputBackendEvent {
    Started,
    Error(anyhow::Error),
    Sample(InputSample),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputSample {
    pub timestamp: Instant,
    /// bitfield
    pub buttons: u32,
    /// `[ 0.0, 1.0 ]`
    pub pressure: f32,
    /// `( 0.0, +inf )`
    pub aspect_ratio: f32,
    /// `[ 0.0, 1.0 ]`
    pub coordinate: Vec2,
}

#[derive(Debug)]
pub struct InputBackendHandle {
    thread: Option<JoinHandle<()>>,
    sample_rx: Option<Receiver<InputBackendEvent>>,
    stop_flag: Arc<AtomicBool>,
    waker: Waker,
}

impl Drop for InputBackendHandle {
    fn drop(&mut self) {
        debug!("Dropping InputBackendHandle.");

        self.sample_rx.take();
        self.stop_flag.store(true, Ordering::Release);
        let _ = self.waker.wake();

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl InputBackendHandle {
    const WAKE_TOKEN: Token = Token(727);

    pub fn create(profile: &Profile) -> Result<Self> {
        let conf = profile.clone();

        let poll = Poll::new()?;
        let waker = Waker::new(poll.registry(), Self::WAKE_TOKEN)?;

        let stop_flag = Arc::new(AtomicBool::new(false));
        let (tx, rx) = flume::unbounded();

        let stop_flag_clone = stop_flag.clone();
        let thread = std::thread::spawn(move || {
            let tx_clone = tx.clone();

            let result = match conf.active_input {
                super::config::InputBackend::Null => null_thread_loop(tx, stop_flag_clone, poll),
                super::config::InputBackend::Net => {
                    net::net_thread_loop(&conf.net_input.listen_addr, tx, stop_flag_clone, poll)
                }
                super::config::InputBackend::LinuxEvdev => {
                    Err(anyhow!("LinuxEvdev not yet implemented."))
                }
                super::config::InputBackend::WindowsWintab => {
                    Err(anyhow!("WindowsWintab not yet implemented."))
                }
            };

            if let Err(err) = result {
                error!("Input thread terminated with error: {err}");
                let _ = tx_clone.send(InputBackendEvent::Error(err));
            } else {
                info!("Input thread terminated normally.");
            }
        });

        let first_event = rx.recv().expect("could not wait for success signal");
        if !matches!(first_event, InputBackendEvent::Started) {
            stop_flag.store(true, Ordering::Release);
            let _ = waker.wake();

            bail!("Did not receive success signal from input thread.");
        }

        Ok(Self {
            thread: Some(thread),
            sample_rx: Some(rx),
            stop_flag,
            waker,
        })
    }

    pub fn get_stop_flag(&self) -> Arc<AtomicBool> {
        self.stop_flag.clone()
    }

    pub fn rx(&self) -> &Receiver<InputBackendEvent> {
        self.sample_rx
            .as_ref()
            .expect("should always be Some() up until destruction")
    }
}

fn null_thread_loop(
    tx: Sender<InputBackendEvent>,
    stop_flag: Arc<AtomicBool>,
    mut poll: Poll,
) -> Result<()> {
    let mut events = Events::with_capacity(2);

    info!("Null input started.");

    tx.send(InputBackendEvent::Started)?;

    while !stop_flag.load(Ordering::Acquire) {
        poll.poll(&mut events, None)?;
    }

    info!("Null input stopping.");

    Ok(())
}
