pub mod net;

use anyhow::Result;
use flume::Receiver;
use slint::JoinHandle;
use std::{sync::{Arc, atomic::AtomicBool}, time::Instant};

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
    sample_rx: Receiver<InputSample>,
    stop_flag: Arc<AtomicBool>,
}

// pub trait InputBackend {
//     fn read(&mut self) -> Result<InputRecord>;

//     fn try_read(&mut self) -> Result<Option<InputRecord>>;
// }
