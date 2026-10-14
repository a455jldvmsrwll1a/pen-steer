/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use anyhow::{Context, Result, bail};
use crossbeam_channel::Sender;
use glam::Vec2;
use log::info;
use mio::{Events, Interest, Poll, Token, net::UdpSocket};
use std::{
    io::ErrorKind,
    net::ToSocketAddrs,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use crate::core::input::{InputBackendEvent, InputBackendHandle, InputSample};

const MESSAGE_SIZE: usize = 20;

pub fn net_thread_loop(
    listen_addr: &str,
    sample_tx: Sender<InputBackendEvent>,
    stop_flag: Arc<AtomicBool>,
    mut poll: Poll,
) -> Result<()> {
    const SOCK_TOKEN: Token = Token(0xDEADBEEF);

    let mut events = Events::with_capacity(128);

    let addr = listen_addr.to_socket_addrs()?.next().unwrap();
    let mut socket = UdpSocket::bind(addr)?;
    poll.registry()
        .register(&mut socket, SOCK_TOKEN, Interest::READABLE)?;

    info!("Bound to UDP socket at {listen_addr}");

    sample_tx.send(InputBackendEvent::Started)?;

    'read: while !stop_flag.load(Ordering::Acquire) {
        poll.poll(&mut events, None)?;

        for event in &events {
            match event.token() {
                InputBackendHandle::WAKE_TOKEN => {
                    if stop_flag.load(Ordering::Acquire) {
                        break 'read;
                    }
                }
                SOCK_TOKEN => {
                    read_packet(&mut socket, &sample_tx).context("failed to read packet")?;
                }
                _ => {}
            }
        }
    }

    info!("Net input stopping!");

    Ok(())
}

fn read_packet(sock: &mut UdpSocket, tx: &Sender<InputBackendEvent>) -> Result<()> {
    let mut buffer = [0u8; MESSAGE_SIZE];
    let len = match sock.recv(&mut buffer) {
        Ok(len) => len,
        Err(err) => match err.kind() {
            ErrorKind::WouldBlock | ErrorKind::TimedOut => return Ok(()),
            _ => bail!("error receiving on socket: {err:?}"),
        },
    };

    if len < MESSAGE_SIZE {
        return Ok(());
    }

    if buffer[0..2] != [0xA4, 0x55] {
        return Ok(());
    }

    // buffer[2] is unused.

    let buttons = buffer[3];
    let pressure = f32::from_be_bytes(buffer[4..8].try_into().unwrap());
    let aspect_ratio = f32::from_be_bytes(buffer[8..12].try_into().unwrap());
    let x = f32::from_be_bytes(buffer[12..16].try_into().unwrap());
    let y = f32::from_be_bytes(buffer[16..20].try_into().unwrap());

    tx.send(InputBackendEvent::Sample(InputSample {
        timestamp: Instant::now(),
        buttons: buttons as u32,
        pressure,
        aspect_ratio,
        coordinate: Vec2::new(x, y),
    }))
    .context("failed to send input sample")?;

    Ok(())
}
