/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

pub mod net;

use anyhow::Result;
use flume::Receiver;
use log::debug;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Instant,
};

use glam::Vec2;

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

pub struct InputBackendHandle {
    thread: JoinHandle<Result<()>>,
    sample_rx: Option<Receiver<InputSample>>,
    stop_flag: Arc<AtomicBool>,
}

impl Drop for InputBackendHandle {
    fn drop(&mut self) {
        debug!("Dropping InputBackendHandle.");

        self.sample_rx.take();
        self.stop_flag.store(true, Ordering::Release);
    }
}

impl InputBackendHandle {
    pub fn create() -> Result<Self> {
        let stop_flag = Arc::new(AtomicBool::new(false));

        let (tx, rx) = flume::unbounded();
        let stop_flag_clone = stop_flag.clone();
        let thread = std::thread::spawn(move || {
            net::net_thread_loop("127.0.0.1:16027", tx, stop_flag_clone)
        });

        Ok(Self {
            thread,
            sample_rx: Some(rx),
            stop_flag,
        })
    }
}
