use std::{
    fmt::Debug,
    fs::{File, OpenOptions},
    os::unix::fs::OpenOptionsExt,
    thread,
    time::Duration,
};

use crate::config::Config;
use anyhow::{Context, Result, bail};
use input_linux::{
    AbsoluteAxis, AbsoluteEvent, AbsoluteInfo, AbsoluteInfoSetup, EventKind, EventTime,
    ForceFeedbackKind, InputEvent, InputId, Key, KeyEvent, KeyState, SynchronizeEvent,
    SynchronizeKind, UInputHandle, sys::BUS_USB,
};
use nix::libc::{O_NONBLOCK, input_event, timeval};

const ZERO: EventTime = EventTime::new(0, 0);
const NULL_EVENT: input_event = input_event {
    time: timeval {
        tv_sec: 0,
        tv_usec: 0,
    },
    type_: 0,
    code: 0,
    value: 0,
};

pub struct UInputDev {
    handle: UInputHandle<File>,
    resolution: f32,
    wheel_axis: Option<i32>,
    horn_key: Option<bool>,
    events_buf: [input_event; 3],
}

impl UInputDev {
    pub fn new(config: &Config) -> Result<Self> {
        if config.device_resolution > u16::MAX as u32 {
            bail!("Device resolution too high!");
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_NONBLOCK)
            .open("/dev/uinput")
            .context("Could not open uinput file!")?;

        let handle = UInputHandle::new(file);

        // Steering wheel horn button.
        handle.set_evbit(EventKind::Key)?;
        handle.set_keybit(Key::ButtonThumbr)?;
        // Unused buttons; can help applications recognise the virtual device.
        handle.set_keybit(Key::ButtonThumbl)?;
        handle.set_keybit(Key::ButtonNorth)?;
        handle.set_keybit(Key::ButtonEast)?;
        handle.set_keybit(Key::ButtonSouth)?;
        handle.set_keybit(Key::ButtonWest)?;

        // Steering wheel absolute axis.
        handle.set_evbit(EventKind::Absolute)?;
        handle.set_absbit(AbsoluteAxis::X)?;
        let abs = AbsoluteInfoSetup {
            axis: AbsoluteAxis::X,
            info: AbsoluteInfo {
                value: 0,
                minimum: -(config.device_resolution as i32),
                maximum: config.device_resolution as i32,
                fuzz: 0,
                flat: 0,
                resolution: config.device_resolution as i32,
            },
        };

        // Advertise force-feedback functionality.
        handle.set_evbit(EventKind::ForceFeedback)?;
        handle.set_ffbit(ForceFeedbackKind::Constant)?;
        // Ignored; just helps with detection.
        handle.set_ffbit(ForceFeedbackKind::Autocenter)?;
        handle.set_ffbit(ForceFeedbackKind::Periodic)?;
        handle.set_ffbit(ForceFeedbackKind::Rumble)?;
        handle.set_ffbit(ForceFeedbackKind::Damper)?;
        handle.set_ffbit(ForceFeedbackKind::Inertia)?;
        handle.set_ffbit(ForceFeedbackKind::Ramp)?;
        handle.set_ffbit(ForceFeedbackKind::Sine)?;
        handle.set_ffbit(ForceFeedbackKind::Square)?;
        handle.set_ffbit(ForceFeedbackKind::Triangle)?;
        handle.set_ffbit(ForceFeedbackKind::SawUp)?;
        handle.set_ffbit(ForceFeedbackKind::SawDown)?;

        let id = InputId {
            bustype: BUS_USB,
            vendor: config.device_vendor,
            product: config.device_product,
            version: config.device_version,
        };

        handle.create(&id, config.device_name.as_bytes(), 10, &[abs])?;

        Ok(Self {
            handle,
            resolution: config.device_resolution as f32,
            wheel_axis: None,
            horn_key: None,
            events_buf: [NULL_EVENT; 3],
        })
    }

    pub fn set_wheel(&mut self, angle: f32) {
        let value = (angle * self.resolution).round_ties_even();
        self.wheel_axis = Some(value as i32);
    }

    pub fn set_horn(&mut self, honking: bool) {
        self.horn_key = Some(honking);
    }

    pub fn apply(&mut self) -> Result<()> {
        let mut i = 0;

        if let Some(axis_val) = self.wheel_axis {
            self.events_buf[i] =
                InputEvent::from(AbsoluteEvent::new(ZERO, AbsoluteAxis::X, axis_val)).into_raw();
            i += 1;
        }

        if let Some(key) = self.horn_key {
            self.events_buf[i] = InputEvent::from(KeyEvent::new(
                ZERO,
                Key::ButtonThumbr,
                KeyState::pressed(key),
            ))
            .into_raw();
            i += 1;
        }

        if i == 0 {
            return Ok(());
        }

        // Insert sync report event.
        self.events_buf[i] =
            InputEvent::from(SynchronizeEvent::new(ZERO, SynchronizeKind::Report, 0)).into_raw();

        self.handle
            .write(&self.events_buf[..=i])
            .context("could not write events")?;

        Ok(())
    }
}

impl Drop for UInputDev {
    fn drop(&mut self) {
        if let Err(err) = self.handle.dev_destroy() {
            eprintln!("Error occured destroying uinput device: {err}");
        }
    }
}

impl Debug for UInputDev {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UInputDev { /* fields */ }")
    }
}
