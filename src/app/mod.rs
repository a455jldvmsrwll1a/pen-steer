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
use log::{error, info};
use pen_steer::{
    PACKAGE_VERSION,
    core::{
        config::Profile,
        controller::{self, Snapshot},
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

    let (quit_tx, quit_rx) = flume::unbounded();
    util::set_handler(quit_tx.clone());

    let (cmd_tx, cmd_rx) = flume::unbounded();
    let (event_tx, event_rx) = rtrb::RingBuffer::new(128);
    let (snapshot_tx, mut snapshot_rx) = triple_buffer::triple_buffer(&Snapshot::default());

    let controller_thread = std::thread::spawn(|| {
        controller::controller(profile, cmd_rx, quit_rx, event_tx, snapshot_tx);
    });

    cmd_tx.send(controller::Command::StartInput)?;
    cmd_tx.send(controller::Command::StartOutput)?;

    if let Err(err) = std::fs::create_dir_all(save_path::save_dir()) {
        error!("Could not create configuration directory: {err:?}");
    }

    let is_running = Arc::new(AtomicBool::default());

    let ui = AppWindow::new()?;
    slint::set_xdg_app_id(APP_ID)?;
    ui.set_app_version(PACKAGE_VERSION.to_shared_string());

    let ui_weak = ui.as_weak();
    ui.global::<ViewTabStatus>().on_fetch_status(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };

        let state = snapshot_rx.read();

        let global = ui.global::<ViewTabStatus>();
        global.set_ffb_available(state.ffb_value.is_some());
        global.set_ffb_value(state.ffb_value.unwrap_or(0.0));
        global.set_steer_value(state.output_state.wheel);
        global.set_horn_pressed(state.output_state.horn);

        match state.pen {
            Some(pen) => {
                global.set_pen_available(true);
                global.set_grip_strength(state.grip_strength);
                global.set_pen_x(pen.position.x);
                global.set_pen_y(pen.position.y);
            }
            None => {
                global.set_pen_available(false);
            }
        }
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

    let is_running_clone = is_running.clone();
    let ui_weak = ui.as_weak();
    ui.on_main_action_clicked(move || {
        is_running_clone.fetch_not(std::sync::atomic::Ordering::SeqCst);

        ui_weak
            .unwrap()
            .invoke_set_running_state(is_running_clone.load(std::sync::atomic::Ordering::SeqCst));
    });

    ui.run()?;

    let _ = quit_tx.send(());

    if let Err(err) = controller_thread.join() {
        error!("Error while waiting for controller thread to stop: {err:?}");
    }

    Ok(())
}
