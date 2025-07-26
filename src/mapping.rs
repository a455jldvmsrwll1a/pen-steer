#[derive(Debug, Clone, Copy)]
pub enum MapOrientation {
    None,
    A90,
    A180,
    A270,
}

#[derive(Debug, Clone)]
pub struct Mapping {
    min_in_x: f32,
    min_in_y: f32,
    max_in_x: f32,
    max_in_y: f32,
    min_out_x: f32,
    min_out_y: f32,
    max_out_x: f32,
    max_out_y: f32,
    orientation: MapOrientation,
    invert_x: bool,
    invert_y: bool,
}

impl Default for Mapping {
    fn default() -> Self {
        Self {
            min_in_x: -1.0,
            min_in_y: -1.0,
            max_in_x: 1.0,
            max_in_y: 1.0,
            min_out_x: -1.0,
            min_out_y: -1.0,
            max_out_x: 1.0,
            max_out_y: 1.0,
            orientation: MapOrientation::None,
            invert_x: false,
            invert_y: false,
        }
    }
}

impl Mapping {
    pub fn transform(&self, x: f32, y: f32) -> (f32, f32) {
        (x, y)
    }
}
