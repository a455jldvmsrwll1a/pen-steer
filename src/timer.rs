use std::time::{Duration, Instant};

pub struct Timer {
    last_tick: Instant,
    period: Duration,
}

impl Timer {
    pub fn new(freq: u32) -> Self {
        Self {
            last_tick: Instant::now(),
            period: Duration::from_secs_f64(1.0 / freq as f64),
        }
    }

    pub fn wait(&mut self) {
        loop {
            let now = Instant::now();
            let elapsed = now - self.last_tick;

            if elapsed >= self.period {
                self.last_tick = now;
                break;
            }

            std::thread::sleep(self.period - elapsed);
        }
    }
}