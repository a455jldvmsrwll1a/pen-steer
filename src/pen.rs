#[derive(Debug, Default, Clone, Copy)]
pub struct Pen {
    pub x: f32,
    pub y: f32,
    pub pressure: u32,
    pub buttons: u8,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RawPen {
    pub x: f32,
    pub y: f32,
    pub pressure: u32,
    pub buttons: u8,
}
