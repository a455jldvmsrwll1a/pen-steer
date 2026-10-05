use anyhow::{Context, Result, bail};
use flume::Sender;
use glam::Vec2;
use log::info;
use std::{
    io::ErrorKind,
    net::UdpSocket,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use crate::core::input::InputSample;

pub fn net_thread_loop(
    listen_addr: &str,
    sample_tx: Sender<InputSample>,
    stop_flag: Arc<AtomicBool>,
) -> Result<()> {
    const MESSAGE_SIZE: usize = 20;

    let mut buffer = [0u8; MESSAGE_SIZE];
    let socket = UdpSocket::bind(listen_addr)?;
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;

    info!("Bound to UDP socket at {listen_addr}");

    loop {
        if stop_flag.load(Ordering::Acquire) {
            info!("Net input stopping!");

            return Ok(());
        }

        let len = match socket.recv(&mut buffer) {
            Ok(len) => len,
            Err(err) => match err.kind() {
                ErrorKind::WouldBlock | ErrorKind::TimedOut => continue,
                _ => bail!("error receiving on socket: {err}"),
            },
        };

        if len < MESSAGE_SIZE {
            continue;
        }

        if buffer[0..2] != [0xA4, 0x55] {
            continue;
        }

        // buffer[2] is unused.

        let buttons = buffer[3];
        let pressure = f32::from_be_bytes(buffer[4..8].try_into().unwrap());
        let aspect_ratio = f32::from_be_bytes(buffer[8..12].try_into().unwrap());
        let x = f32::from_be_bytes(buffer[12..16].try_into().unwrap());
        let y = f32::from_be_bytes(buffer[16..20].try_into().unwrap());

        sample_tx
            .send(InputSample {
                timestamp: Instant::now(),
                buttons: buttons as u32,
                pressure,
                aspect_ratio,
                coordinate: Vec2::new(x, y),
            })
            .context("failed to send input sample")?;
    }
}
