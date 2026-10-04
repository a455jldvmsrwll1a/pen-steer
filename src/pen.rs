use std::{marker::PhantomData, time::Instant};

use crate::math::Vec2;

#[derive(Debug, Clone, Copy)]
pub struct PreTransform;
#[derive(Debug, Clone, Copy)]
pub struct PostTransform;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pen<S> {
    pub timestamp: Instant,
    /// bitfield
    pub buttons: u8,
    /// `[ 0.0, 1.0 ]`
    pub pressure: f32,
    /// `( 0.0, +inf )`
    pub aspect_ratio: f32,
    pub position: Vec2,
    _phantom: PhantomData<S>,
}

impl<S> Default for Pen<S> {
    fn default() -> Self {
        Self {
            timestamp: Instant::now(),
            buttons: 0,
            pressure: 0.0,
            aspect_ratio: 1.0,
            position: Vec2::default(),
            _phantom: PhantomData::<S>::default(),
        }
    }
}

impl<S> Pen<S> {
    pub fn velocity(&self, other: &Pen<S>) -> Vec2 {
        let dt = (self.timestamp - other.timestamp).as_secs_f32();

        if dt > 0.0 {
            (self.position - other.position) / dt
        } else {
            Vec2::ZERO
        }
    }
}
