/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

mod uinput;

use std::{cell::Cell, ops::DerefMut};

use anyhow::{Result, bail};
use log::debug;

use crate::core::{config::Profile, output::uinput::UInputBackend};

pub trait OutputBackend {
    /// Get the current feedback force from the output device.
    ///
    /// # Returns
    ///
    /// The feedback force (in no particular unit), or `None` if unavailable:
    ///
    /// - The output backend or device does not support force-feedback.
    ///
    /// - Force-feedback is currently not being used by any process.
    ///
    fn get_feedback_force(&self) -> Option<f32>;

    /// Set the wheel axis value.
    ///
    /// Full range is [-1.0, 1.0], in no particular unit.
    ///
    fn set_wheel(&mut self, normalised: f32);

    /// Set the accelerator axis value.
    ///
    /// Full range is [0.0, 1.0], in no particular unit.
    ///
    fn set_accelerator(&mut self, normalised: f32);

    /// Set the brake axis value.
    ///
    /// Full range is [0.0, 1.0], in no particular unit.
    ///
    fn set_brake(&mut self, normalised: f32);

    /// Set the horn's state.
    ///
    /// `true` if the horn is pressed, and `false` if not.
    ///
    fn set_horn(&mut self, honking: bool);

    /// Apply any changes that were made to the actual output mechanism (e.g. uinput or vigem).
    ///
    fn apply(&mut self) -> Result<()>;

    /// Handle any events that the backend needs to respond to (such as receiving force-feedback events).
    ///
    fn handle_events(&mut self);
}

pub fn create_output(profile: &Profile) -> Result<Box<dyn OutputBackend>> {
    Ok(match profile.active_output {
        super::config::OutputBackend::Null => Box::new(DummyOutputBackend),
        super::config::OutputBackend::LinuxUinput => Box::new(UInputBackend::new(profile)?),
        super::config::OutputBackend::LinuxUsbPeripheral => bail!("Not yet implemented."),
        super::config::OutputBackend::WindowsVigemBus => bail!("Not yet implemented."),
        super::config::OutputBackend::UsbOverIpServer => bail!("Not yet implemented."),
    })
}

// Convenience wrapper. Operations are no-op when output is absent.
impl<B: ?Sized + OutputBackend> OutputBackend for Option<&mut B> {
    fn get_feedback_force(&self) -> Option<f32> {
        self.as_ref().map(|out| out.get_feedback_force()).flatten()
    }

    fn set_wheel(&mut self, normalised: f32) {
        if let Some(out) = self {
            out.set_wheel(normalised);
        }
    }

    fn set_accelerator(&mut self, normalised: f32) {
        if let Some(out) = self {
            out.set_accelerator(normalised);
        }
    }

    fn set_brake(&mut self, normalised: f32) {
        if let Some(out) = self {
            out.set_brake(normalised);
        }
    }

    fn set_horn(&mut self, honking: bool) {
        if let Some(out) = self {
            out.set_horn(honking);
        }
    }

    fn apply(&mut self) -> Result<()> {
        self.as_mut().map(|out| out.apply()).unwrap_or(Ok(()))
    }

    fn handle_events(&mut self) {
        if let Some(out) = self {
            out.handle_events();
        }
    }
}

/// yeah this is hella ass imho
pub struct OutputWrapperThing {
    pub inner: Box<dyn OutputBackend>,
    pub last_ffb: Cell<Option<f32>>,
    pub last_wheel: f32,
    pub last_horn: bool,
}

impl OutputWrapperThing {
    pub fn new(inner: Box<dyn OutputBackend>) -> Self {
        Self {
            inner,
            last_ffb: Cell::new(None),
            last_wheel: 0.0,
            last_horn: false,
        }
    }
}

impl OutputBackend for OutputWrapperThing {
    fn get_feedback_force(&self) -> Option<f32> {
        let ffb = self.inner.get_feedback_force();
        self.last_ffb.set(ffb);
        
        ffb
    }

    fn set_wheel(&mut self, normalised: f32) {
        self.last_wheel = normalised;
        self.inner.set_wheel(normalised);
    }

    fn set_accelerator(&mut self, normalised: f32) {
        self.inner.set_accelerator(normalised);
    }

    fn set_brake(&mut self, normalised: f32) {
        self.inner.set_brake(normalised);
    }

    fn set_horn(&mut self, honking: bool) {
        if self.last_horn != honking {
            debug!("set_horn({honking})");
        }

        self.last_horn = honking;
        self.inner.set_horn(honking);
    }

    fn apply(&mut self) -> Result<()> {
        self.inner.apply()
    }

    fn handle_events(&mut self) {
        self.inner.handle_events();
    }
}

pub struct DummyOutputBackend;

impl OutputBackend for DummyOutputBackend {
    fn get_feedback_force(&self) -> Option<f32> {
        None
    }

    fn set_wheel(&mut self, _angle: f32) {}

    fn set_accelerator(&mut self, _normalised: f32) {}

    fn set_brake(&mut self, _normalised: f32) {}

    fn set_horn(&mut self, _honking: bool) {}

    fn apply(&mut self) -> Result<()> {
        Ok(())
    }

    fn handle_events(&mut self) {}
}
