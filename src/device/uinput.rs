use std::{
    fmt::Debug,
    fs::{File, OpenOptions},
    os::unix::fs::OpenOptionsExt,
};

use crate::config::Config;
use anyhow::{Context, Result, bail};
use input_linux::{
    AbsoluteAxis, AbsoluteInfo, AbsoluteInfoSetup, EventKind, ForceFeedbackKind, InputId, Key,
    UInputHandle,
    sys::BUS_USB,
};
use nix::libc::O_NONBLOCK;

pub struct UInputDev {
    handle: UInputHandle<File>,
}

impl UInputDev {
    pub fn new(config: &Config) -> Result<Self> {
        if config.device_resolution > i32::MAX as u32 {
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

        Ok(Self { handle })
    }
}

impl Debug for UInputDev {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UInputDev { /* fields */ }")
    }
}
