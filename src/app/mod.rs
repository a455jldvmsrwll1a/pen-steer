/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

pub mod save_path;
pub mod slint_sync;
pub mod util;

use std::sync::{Arc, atomic::AtomicBool};

use anyhow::Result;
use arc_swap::ArcSwapAny;
use crossbeam_channel::RecvError;
use log::{debug, error, info};
use pen_steer::{
    PACKAGE_VERSION,
    core::{
        config::Profile,
        controller::{self, Snapshot},
        output::FFbState,
    },
};
use slint::{ComponentHandle, ToSharedString};

use crate::{APP_ID, AboutWindow, AppWindow, GlobalUiState, SlintWindow, ViewTabStatus};

pub fn start_app() -> Result<()> {
    let profile = Arc::new(ArcSwapAny::new(Arc::new(Profile {
        active_input: pen_steer::core::config::InputBackend::Net,
        active_output: pen_steer::core::config::OutputBackend::LinuxUinput,
        ..Default::default()
    })));

    let (quit_tx, quit_rx) = crossbeam_channel::unbounded();
    util::set_handler(quit_tx.clone());

    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (event_tx, event_rx) = crossbeam_channel::bounded(128);
    let (snapshot_tx, mut snapshot_rx) = triple_buffer::triple_buffer(&Snapshot::default());

    let controller_thread = std::thread::spawn(|| {
        controller::controller(profile, cmd_rx, quit_rx, event_tx, snapshot_tx);
    });

    if let Err(err) = std::fs::create_dir_all(save_path::save_dir()) {
        error!("Could not create configuration directory: {err:?}");
    }

    let ui = AppWindow::new()?;
    slint::set_xdg_app_id(APP_ID)?;
    ui.set_app_version(PACKAGE_VERSION.to_shared_string());

    let ui_weak = ui.as_weak();
    let relay_thread = std::thread::spawn(move || {
        'read: loop {
            match event_rx.recv() {
                Ok(controller::Event::Error(err)) => {
                    error!("Received error from controller thread:\n{err:?}");
                }
                Ok(controller::Event::InputStarted) => {
                    info!("Controller thread says input started!");

                    let _ = ui_weak.upgrade_in_event_loop(|ui| {
                        ui.invoke_set_running_state(true);
                        ui.invoke_on_input_started();
                    });
                }
                Ok(controller::Event::OutputStarted) => {
                    info!("Controller thread says output started!");

                    let _ = ui_weak.upgrade_in_event_loop(|ui| {
                        ui.invoke_set_running_state(true);
                        ui.invoke_on_output_started();
                    });
                }
                Err(RecvError) => break 'read,
            }
        }

        debug!("Relay thread stopping!");
    });

    let ui_weak = ui.as_weak();
    ui.global::<ViewTabStatus>().on_fetch_status(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };

        let state = snapshot_rx.read();

        let global = ui.global::<ViewTabStatus>();

        match state.ffb_state {
            FFbState::Active { constant_force, .. } => {
                global.set_ffb_available(true);
                global.set_ffb_value(constant_force);
            }
            FFbState::Unavailable | FFbState::Idle { .. } => {
                global.set_ffb_available(false);
            }
        }

        global.set_steer_value(state.output_state.wheel);
        global.set_horn_pressed(state.output_state.horn);

        global.set_pen_available(true);
        global.set_grip_strength(state.grip_strength);
        global.set_pen_x(state.pen.position.x);
        global.set_pen_y(state.pen.position.y);
    });

    let ui_weak = ui.as_weak();
    ui.global::<GlobalUiState>().on_ctl_config_changed(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };

        let global = ui.global::<GlobalUiState>();

        info!("control config updated!");
        eprintln!("{:#?}", global.get_control_config());
    });

    ui.on_show_about_app(move || {
        if let Ok(window) = AboutWindow::new() {
            let _ = window.run();
        }
    });

    ui.on_show_about_slint(move || {
        if let Ok(window) = SlintWindow::new() {
            let _ = window.run();
        }
    });

    let cmd_tx_clone = cmd_tx.clone();
    ui.on_reset_wheel_state_clicked(move || {
        if let Err(err) = cmd_tx_clone.send(controller::Command::ResetControlState) {
            error!("Error trying to send control reset command: {err:?}");
        }
    });

    let cmd_tx_clone = cmd_tx.clone();
    let ui_weak = ui.as_weak();
    ui.on_main_action_clicked(move |was_indicated_running| {
        let inner = || -> Result<()> {
            if was_indicated_running {
                cmd_tx_clone.send(controller::Command::StopInput)?;
                cmd_tx_clone.send(controller::Command::StopOutput)?;

                ui_weak.unwrap().invoke_set_running_state(false);
            } else {
                cmd_tx_clone.send(controller::Command::StartInput)?;
                cmd_tx_clone.send(controller::Command::StartOutput)?;
            };
            Ok(())
        };

        if let Err(err) = inner() {
            error!("Error trying to send start/stop command: {err:?}");
        }
    });

    ui.run()?;

    let _ = quit_tx.send(());

    if let Err(err) = controller_thread.join() {
        error!("Error while waiting for controller thread to stop: {err:?}");
    }

    if let Err(err) = relay_thread.join() {
        error!("Error while waiting for relay thread to stop: {err:?}");
    }

    Ok(())
}
