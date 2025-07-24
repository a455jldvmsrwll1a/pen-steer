use anyhow::Result;
use log::info;
use std::net::UdpSocket;

use crate::{pen::Pen, source::Source};

#[derive(Debug)]
pub struct NetSource {
    socket: UdpSocket,
    pub aspect_ratio: f32,
}

impl NetSource {
    pub fn new(addr: &str) -> Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;

        info!("Bound to {addr}");

        Ok(Self {
            socket,
            aspect_ratio: 1.0,
        })
    }
}

impl Source for NetSource {
    fn get(&mut self) -> Option<Pen> {
        let mut pen = Pen::default();
        let mut buf = [0u8; 13];
        let mut filled = false;

        loop {
            let Some((len, _)) = self.socket.recv_from(&mut buf).ok() else {
                return filled.then_some(pen);
            };

            if len != 13 {
                return filled.then_some(pen);
            }

            filled = true;
            pen.x = f32::from_le_bytes(buf[0..4].try_into().unwrap());
            pen.y = f32::from_le_bytes(buf[4..8].try_into().unwrap());
            pen.pressure = u32::from_le_bytes(buf[8..12].try_into().unwrap());
            pen.buttons = buf[12];
        }
    }
}
