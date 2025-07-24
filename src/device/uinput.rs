use std::{
    fmt::Debug,
    fs::{File, OpenOptions},
    os::unix::fs::OpenOptionsExt,
};

use crate::{config::Config, device::Device};
use anyhow::{Context, Result, bail};
use input_linux::{
    AbsoluteAxis, AbsoluteEvent, AbsoluteInfo, AbsoluteInfoSetup, EventKind, EventTime,
    ForceFeedbackKind, InputEvent, InputId, Key, KeyEvent, KeyState, SynchronizeEvent,
    SynchronizeKind, UInputHandle,
    sys::{
        BUS_USB, EV_FF, EV_UINPUT, FF_CONSTANT, FF_GAIN, UI_FF_ERASE, UI_FF_UPLOAD,
        uinput_ff_erase, uinput_ff_upload,
    },
};
use log::{debug, error, info, trace};
use nix::libc::{
    O_NONBLOCK, ff_constant_effect, ff_effect, ff_replay, ff_trigger, input_event, timeval,
};

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
const NULL_EFFECT: ff_effect = ff_effect {
    type_: 0,
    id: 0,
    direction: 0,
    trigger: ff_trigger {
        button: 0,
        interval: 0,
    },
    replay: ff_replay {
        length: 0,
        delay: 0,
    },
    u: [0u64; 4],
};

#[derive(Default, Clone, Copy)]
struct FFState {
    request_id: u32,
    playing: bool,
    force: i16,
}

pub struct UInputDevice {
    handle: UInputHandle<File>,
    resolution: f32,
    wheel_axis: Option<i32>,
    horn_key: Option<bool>,
    ff: Option<FFState>,
    events_buf: [input_event; 3],
}

impl UInputDevice {
    pub fn new(config: &Config) -> Result<Self> {
        if config.device_resolution > u16::MAX as u32 {
            bail!("Device resolution too high!");
        }

        if config.device_name.is_empty() {
            bail!("Empty device name is prohibited!");
        }

        if config.device_name.len() >= 80 {
            bail!("Device name can be up to 79 characters only!");
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

        debug!(
            "Creating virtual device:\n\tName: {}\n\tVendor: 0x{:X}\n\tProduct: 0x{:X}\n\tVersion: 0x{:X}",
            config.device_name, config.device_vendor, config.device_product, config.device_version
        );

        handle.create(&id, config.device_name.as_bytes(), 10, &[abs])?;

        info!("Initialised!");

        Ok(Self {
            handle,
            resolution: config.device_resolution as f32,
            wheel_axis: None,
            horn_key: None,
            ff: None,
            events_buf: [NULL_EVENT; 3],
        })
    }

    fn handle_ff_upload(&mut self, request_id: u32) -> Result<()> {
        let mut upload = uinput_ff_upload {
            request_id,
            retval: 0,
            effect: NULL_EFFECT,
            old: NULL_EFFECT,
        };

        self.handle
            .ff_upload_begin(&mut upload)
            .context("could not begin ff upload")?;

        if upload.effect.type_ == FF_CONSTANT {
            if self.ff.is_none() {
                debug!("Force-feedback active.");
            }

            let ff = self.ff.get_or_insert_default();
            ff.request_id = request_id;

            // SAFETY: the effect type is checked before accessing the union.
            unsafe {
                let constant = &*(upload.effect.u.as_ptr() as *const ff_constant_effect);
                ff.force = constant.level;
                trace!("ff = {}", constant.level);
            }
        }

        self.handle
            .ff_upload_end(&upload)
            .context("could not end ff upload")?;

        Ok(())
    }

    fn handle_ff_erase(&mut self, request_id: u32) -> Result<()> {
        let mut erase = uinput_ff_erase {
            request_id,
            retval: 0,
            effect_id: 0,
        };

        self.handle
            .ff_erase_begin(&mut erase)
            .context("could not begin ff erase")?;

        if let Some(state) = self.ff
            && erase.effect_id == state.request_id
        {
            self.ff = None;
            debug!("Force-feedback inactive.");
        }

        self.handle
            .ff_erase_end(&erase)
            .context("could not end ff erase")?;

        Ok(())
    }
}

impl Device for UInputDevice {
    fn get_feedback(&self) -> f32 {
        let Some(ff) = self.ff else {
            return 0.0;
        };

        if !ff.playing {
            return 0.0;
        }

        ff.force as f32 / i16::MAX as f32
    }

    fn set_wheel(&mut self, angle: f32) {
        let value = (angle * self.resolution).round_ties_even();
        self.wheel_axis = Some(value as i32);
    }

    fn set_horn(&mut self, honking: bool) {
        self.horn_key = Some(honking);
    }

    fn apply(&mut self) -> Result<()> {
        let mut i = 0;

        if let Some(axis_val) = self.wheel_axis.take() {
            self.events_buf[i] =
                InputEvent::from(AbsoluteEvent::new(ZERO, AbsoluteAxis::X, axis_val)).into_raw();
            i += 1;
        }

        if let Some(key) = self.horn_key.take() {
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

    fn handle_events(&mut self) {
        let mut ev = NULL_EVENT;

        while let Ok(1) = self.handle.read(std::slice::from_mut(&mut ev)) {
            match ev.type_ as i32 {
                EV_UINPUT => match ev.code as i32 {
                    UI_FF_UPLOAD => {
                        if let Err(err) = self.handle_ff_upload(ev.value as u32) {
                            error!("Error handling ff upload: {err}");
                        }
                    }
                    UI_FF_ERASE => {
                        if let Err(err) = self.handle_ff_erase(ev.value as u32) {
                            error!("Error handling ff erase: {err}");
                        }
                    }
                    _ => {
                        error!("Unrecognised EV_UINPUT code {}.", ev.code);
                    }
                },
                EV_FF => {
                    if let Some(state) = &mut self.ff {
                        // TODO: what does ev.code really do???
                        match ev.code {
                            0 => state.playing = ev.value != 0,
                            FF_GAIN => debug!("FF_GAIN = {}", ev.value),
                            n => debug!("Unexpected EV_FF code {n}."),
                        }
                    }
                }
                _ => {
                    debug!("Unexpected event type {}.", ev.type_);
                }
            }
        }
    }
}

impl Drop for UInputDevice {
    fn drop(&mut self) {
        if let Err(err) = self.handle.dev_destroy() {
            error!("Error occured destroying uinput device: {err}");
        }
    }
}

impl Debug for UInputDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UInputDev { /* fields */ }")
    }
}
