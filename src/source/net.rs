use anyhow::Result;
use log::info;
use std::net::UdpSocket;

use crate::{pen::RawPen, source::Source};

#[derive(Debug)]
pub struct NetSource {
    socket: UdpSocket,
}

impl NetSource {
    pub fn new(addr: &str) -> Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;

        info!("Bound to {addr}");

        Ok(Self {
            socket,
        })
    }
}

impl Source for NetSource {
    fn get(&mut self) -> Option<RawPen> {
        let mut pen = RawPen::default();
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
