use std::sync::{Arc, Mutex};

use crate::{state::State, timer::Timer};

pub fn controller(state: Arc<Mutex<State>>) {
    let mut timer = Timer::new(state.lock().unwrap().config.update_frequency);

    loop {
        timer.wait();
    }
}
