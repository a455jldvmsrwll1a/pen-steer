use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pen {
    pub timestamp: Instant,
    pub pressure: u32,
    pub buttons: u8,
    pub x: f32,
    pub y: f32,
    // pub 
}

impl Default for Pen {
    fn default() -> Self {
        Self {
            timestamp: Instant::now(),
            pressure: 0,
            buttons: 0,
            x: 0.0,
            y: 0.0,
        }
    }
}
