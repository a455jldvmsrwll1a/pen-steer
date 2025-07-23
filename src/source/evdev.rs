use std::fs::{self, OpenOptions};

use anyhow::Result;
use input_linux::{AbsoluteAxis, EventKind};

use crate::pen::Pen;

#[derive(Debug)]
pub struct EvdevSource {}

impl EvdevSource {
    pub fn new(preferred_device_name: Option<&str>) -> Result<Self> {
        Ok(Self {})
    }

    pub fn try_read(&mut self) -> Option<Pen> {
        None
    }
}

pub fn enumerate_available_devices() -> Result<Vec<String>> {
    let mut valid_devices = vec![];

    for entry in fs::read_dir("/dev/input/")? {
        let Ok(entry) = entry else {
            continue;
        };

        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };

        let stripped_name = name.trim_start_matches("event");

        if stripped_name.parse::<u32>().is_err() {
            continue;
        }

        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() || file_type.is_file() {
            continue;
        }

        let Ok(file) = OpenOptions::new().read(true).open(entry.path()) else {
            continue;
        };

        let handle = input_linux::EvdevHandle::new(file);

        let Ok(events) = handle.event_bits() else {
            continue;
        };

        if !events.iter().any(|e| matches!(e, EventKind::Absolute)) {
            continue;
        }

        let Ok(abs) = handle.absolute_bits() else {
            continue;
        };

        let mut has_x = false;
        let mut has_y = false;
        let mut has_pressure = false;

        for abs in &abs {
            match abs {
                AbsoluteAxis::X => has_x = true,
                AbsoluteAxis::Y => has_y = true,
                AbsoluteAxis::Pressure => has_pressure = true,
                _ => (),
            }
        }

        if !has_x || !has_y || !has_pressure {
            continue;
        }

        let Ok(dev_name) = handle.device_name() else {
            continue;
        };

        let string = String::from_utf8_lossy(&dev_name).into_owned();
        valid_devices.push(string);
    }

    Ok(valid_devices)
}
