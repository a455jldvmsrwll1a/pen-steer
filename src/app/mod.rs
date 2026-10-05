pub mod save_path;
pub mod util;

use std::sync::{Arc, atomic::AtomicBool};

use anyhow::Result;
use log::{error, info};
use pen_steer::{
    PACKAGE_VERSION,
    core::{config::Mapping, transform::Transformer},
};
use slint::{ComponentHandle, ToSharedString};

use crate::{APP_ID, AboutWindow, AppWindow, GlobalUiState, SlintWindow};

pub fn start_app() -> Result<()> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    util::set_handler(stop_flag.clone());

    let (sample_tx, sample_rx) = flume::unbounded();
    let stop_flag_clone = stop_flag.clone();
    let handle = std::thread::spawn(move || {
        pen_steer::core::input::net::net_thread_loop("127.0.0.1:16027", sample_tx, stop_flag_clone)
    });
    let mut transformer = Transformer::new(&Mapping::default());

    while let Ok(sample) = sample_rx.recv() {
        transformer.update(sample);
        let pen = transformer.get_pen();

        eprintln!(
            "btn: {:#08b}, press: {:5.1}%, aspect: {:.3}, x: {:4.2}, y: {:4.2}, X: {:5.2}, Y: {:5.2}, X': {:5.2}, Y': {:5.2}",
            sample.buttons,
            sample.pressure * 100.0,
            sample.aspect_ratio,
            sample.coordinate.x,
            sample.coordinate.y,
            pen.position.x,
            pen.position.y,
            pen.velocity.x,
            pen.velocity.y,
        );
    }

    info!("thread stopped:{:?}", handle.join());

    return Ok(());

    if let Err(err) = std::fs::create_dir_all(save_path::save_dir()) {
        error!("Could not create configuration directory: {err}");
    }

    let is_running = Arc::new(AtomicBool::default());

    let ui = AppWindow::new()?;
    slint::set_xdg_app_id(APP_ID)?;
    ui.set_app_version(PACKAGE_VERSION.to_shared_string());

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

    Ok(())
}
