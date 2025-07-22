use std::net::SocketAddr;

#[derive(Debug)]
pub struct NetSource {
    listen: SocketAddr,
    aspect_ratio: f32,
}
