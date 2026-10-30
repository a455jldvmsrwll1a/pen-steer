/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#![cfg(target_os = "windows")]

use anyhow::{Result, anyhow};

use crate::core::{config::WindowsVigemBusOutput, output::OutputBackendStuffs};

pub fn thread_loop(_config: &WindowsVigemBusOutput, _obs: OutputBackendStuffs) -> Result<()> {
    Err(anyhow!("WindowsVigemBus not yet implemented."))
}
