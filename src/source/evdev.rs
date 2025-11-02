use std::{
    fmt::Debug,
    fs::{self, DirEntry, File, OpenOptions},
    os::unix::fs::OpenOptionsExt,
};

use anyhow::{Context, Result, bail};
use input_linux::{AbsoluteAxis, EvdevHandle, EventKind, EventRef};
use log::{debug, info, trace};
use nix::libc::O_NONBLOCK;

use crate::{pen::RawPen, source::Source};

pub struct EvdevSource {
    handle: EvdevHandle<File>,
    x_min: i32,
    x_max: i32,
    y_min: i32,
    y_max: i32,
    aspect_ratio: f32,
    current: RawPen,
}

impl EvdevSource {
    pub fn new(preferred_device_name: Option<&str>) -> Result<Self> {
        let device_name;

        if let Some(dev) = preferred_device_name {
            device_name = dev.to_string();
        } else {
            debug!("No source device preference.");
            let devices = enumerate_available_devices()?;
            if let Some(first) = devices.first() {
                device_name = first.clone();
            } else {
                bail!("No valid input devices available! (evdev)");
            }
        }

        debug!("Using source device: {device_name}");

        let Some(handle) =
            open_device_with_name(&device_name).context("Failed to open evdev device.")?
        else {
            bail!("No such device found.");
        };

        let (x_min, x_max, y_min, y_max) = get_dimensions(&handle)?;
        let width = x_max - x_min;
        let height = y_max - y_min;
        let aspect_ratio = width as f32 / height as f32;

        debug!(
            "\nArea:\n\tx-axis: {x_min} .. {x_max}\n\ty-axis: {y_min} .. {y_max}\naspect ratio: {aspect_ratio}"
        );

        info!("Initialised!");

        Ok(Self {
            handle,
            x_min,
            x_max,
            y_min,
            y_max,
            aspect_ratio,
            current: RawPen::default(),
        })
    }
}

impl Source for EvdevSource {
    fn get(&mut self) -> Option<RawPen> {
        fn norm(t: i32, a1: i32, a2: i32) -> f32 {
            ((-1.0) + (t as f64 - a1 as f64) * (1.0 - (-1.0)) / (a2 as f64 - a1 as f64)) as f32
        }

        let mut changed = false;

        while let Ok(event) = self.handle.read_input_event() {
            let Ok(event) = EventRef::new(&event) else {
                continue;
            };

            let EventRef::Absolute(abs) = event else {
                continue;
            };

            match abs.axis {
                AbsoluteAxis::X => {
                    self.current.x = norm(abs.value, self.x_min, self.x_max);
                    if self.aspect_ratio > 1.0 {
                        self.current.x = (self.current.x * self.aspect_ratio).clamp(-1.0, 1.0);
                    }
                    changed = true;
                }
                AbsoluteAxis::Y => {
                    self.current.y = norm(abs.value, self.y_min, self.y_max);
                    if self.aspect_ratio < 1.0 {
                        self.current.y =
                            (self.current.y * (1.0 / self.aspect_ratio)).clamp(-1.0, 1.0);
                    }
                    changed = true;
                }
                AbsoluteAxis::Pressure => {
                    self.current.pressure = abs.value.max(0) as u32;
                    changed = true;
                }
                _ => {}
            }
        }

        changed.then_some(self.current.clone())
    }
}

impl Debug for EvdevSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UInputDev { /* fields */ }")
    }
}

pub fn enumerate_available_devices() -> Result<Vec<String>> {
    let mut valid_devices = vec![];

    for entry in fs::read_dir("/dev/input/")? {
        let Ok(entry) = entry else {
            continue;
        };

        let name = entry.file_name();
        let handle = match open_evdev_tablet_device(entry) {
            Ok(h) => h,
            Err(err) => {
                trace!("Skipping {name:?}: {err}");
                continue;
            }
        };
        
        trace!("Found valid input: {}", handle.name);
        valid_devices.push(handle.name);
    }

    Ok(valid_devices)
}

fn open_device_with_name(target_name: &str) -> Result<Option<EvdevHandle<File>>> {
    for entry in fs::read_dir("/dev/input/")? {
        let Ok(entry) = entry else {
            continue;
        };

        let name = entry.file_name();
        let handle = match open_evdev_tablet_device(entry) {
            Ok(h) => h,
            Err(err) => {
                trace!("Skipping {name:?}: {err}");
                continue;
            }
        };

        if handle.name.contains(target_name) {
            return Ok(Some(handle.handle));
        }
    }

    Ok(None)
}

struct EvdevDeviceHandle {
    handle: EvdevHandle<File>,
    name: String,
}

fn open_evdev_tablet_device(entry: DirEntry) -> Result<EvdevDeviceHandle> {
    let Ok(name) = entry.file_name().into_string() else {
        bail!("Invalid UTF-8 for entry: {:?}", entry.file_name());
    };

    let stripped_name = name.trim_start_matches("event");
    stripped_name
        .parse::<u32>()
        .context("Not a valid event device file.")?;

    let file_type = entry.file_type()?;
    if file_type.is_dir() || file_type.is_file() {
        bail!("Not a device file.");
    }

    let file = OpenOptions::new()
        .read(true)
        .custom_flags(O_NONBLOCK)
        .open(entry.path())?;

    let handle = input_linux::EvdevHandle::new(file);

    let events = handle.event_bits()?;

    if !events.iter().any(|e| matches!(e, EventKind::Absolute)) {
        bail!("No absolute event type.");
    }

    let abs = handle.absolute_bits()?;

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
        bail!("Input device must have X, Y, and pressure axes.");
    }

    let dev_name = handle.device_name()?;
    let name = String::from_utf8_lossy(&dev_name).into_owned();

    Ok(EvdevDeviceHandle { handle, name })
}

fn get_dimensions(handle: &EvdevHandle<File>) -> Result<(i32, i32, i32, i32)> {
    let info_x = handle
        .absolute_info(AbsoluteAxis::X)
        .context("Could not get X axis info.")?;
    let info_y = handle
        .absolute_info(AbsoluteAxis::Y)
        .context("Could not get Y axis info.")?;

    Ok((
        info_x.minimum,
        info_x.maximum,
        info_y.minimum,
        info_y.maximum,
    ))
}
