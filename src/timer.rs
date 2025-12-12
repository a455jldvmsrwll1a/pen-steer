use std::time::{Duration, Instant};

pub struct Timer {
    next_tick: Instant,
    period: Duration,
}

impl Timer {
    pub fn new(freq: u32) -> Self {
        let now = Instant::now();
        let period = Duration::from_secs_f64(1.0 / freq as f64);

        Self {
            next_tick: now + period,
            period,
        }
    }

    pub fn wait(&mut self) {
        loop {
            let now = Instant::now();

            if now >= self.next_tick {
                break;
            }

            std::thread::sleep(self.next_tick - now);
        }

        self.next_tick += self.period;
    }
}
