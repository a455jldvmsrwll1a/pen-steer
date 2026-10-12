/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::time::{Duration, Instant};

pub struct Timer {
    period: Duration,
    next_tick: Instant,
}

impl Timer {
    pub fn with_frequency(freq: f64) -> Self {
        if freq != 0.0 {
            Self::with_period(Duration::from_secs_f64(freq.recip()))
        } else {
            Self::with_period(Duration::MAX)
        }
    }

    pub fn with_period(period: Duration) -> Self {
        Self {
            period,
            next_tick: Instant::now() + period,
        }
    }

    pub fn wait_for_next_tick(&mut self) {
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
