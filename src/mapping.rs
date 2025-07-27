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
    pub fn transform(&self, mut x: f32, mut y: f32) -> (f32, f32) {
        x = inv_lerp(x, self.min_in_x, self.max_in_x).clamp(0.0, 1.0);
        y = inv_lerp(y, self.min_in_y, self.max_in_y).clamp(0.0, 1.0);

        if self.invert_x {
            x = 1.0 - x;
        }

        if self.invert_y {
            y = 1.0 - y;
        }

        x = lerp(x, self.min_out_x, self.max_out_x).clamp(-1.0, 1.0);
        y = lerp(y, self.min_out_y, self.max_out_y).clamp(-1.0, 1.0);

        match self.orientation {
            MapOrientation::None => (x, y),
            MapOrientation::A90 => (-y, x),
            MapOrientation::A180 => (-x, -y),
            MapOrientation::A270 => (y, -x),
        }
    }
}

fn lerp(t: f32, b1: f32, b2: f32) -> f32 {
    b1 + t * (b2 - b1)
}

fn inv_lerp(t: f32, a1: f32, a2: f32) -> f32 {
    (t - a1) / (a2 - a1)
}
