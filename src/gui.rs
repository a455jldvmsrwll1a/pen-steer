use std::sync::{Arc, Mutex};

use crate::{
    config::{self, Config},
    pen::Pen,
    save::{compile_parse_errors, load_file, save_file},
    save_path::{save_dir, save_path},
    state::State,
};
use anyhow::anyhow;
use eframe::egui::{
    self, Color32, CornerRadius, Id, Pos2, Rect, RichText, Sense, Stroke, Vec2, ViewportBuilder,
};
use log::{debug, error};

pub struct GuiApp {
    state: Arc<Mutex<State>>,
    evdev_available_devices: Option<Vec<String>>,
    dirty_source_config: bool,
    dirty_device_config: bool,
    flash_cooldown: f32,
    flash_state: bool,
}

impl GuiApp {
    pub fn new(state: Arc<Mutex<State>>, _cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            state,
            evdev_available_devices: None,
            dirty_source_config: false,
            dirty_device_config: false,
            flash_cooldown: 0.0,
            flash_state: false,
        }
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let mut state2 = self.state.lock().unwrap();
        if state2.gui_context.is_none() {
            state2.gui_context = Some(ctx.clone());
        }
        let mut config = state2.config.clone();
        let mut wheel = state2.wheel.clone();
        let pen = state2.pen_override.clone().or_else(|| state2.pen.clone());
        let mut pen_override = None;
        let reported_error = state2.last_error.take();
        drop(state2);

        if let Some(err) = reported_error {
            error!("\n* * * * * * * * * *\n{err:?}\n* * * * * * * * * *");

            let _ = native_dialog::MessageDialogBuilder::default()
                .set_level(native_dialog::MessageLevel::Error)
                .set_title("Pen Steer: Controller Error")
                .set_owner(frame)
                .set_text(format!("{err:?}"))
                .alert()
                .show();
        }

        let mut dirty_wheel = false;
        let mut dirty_config = false;
        let mut load_path = None;
        let mut should_save = false;

        self.flash_cooldown -= ctx.input(|i| i.unstable_dt);
        if self.flash_cooldown <= 0.0 {
            self.flash_cooldown = 1.0 / 3.0;
            self.flash_state = !self.flash_state;
        }

        if (self.dirty_source_config || self.dirty_device_config) && !ctx.has_requested_repaint() {
            ctx.request_repaint_after_secs(self.flash_cooldown);
        }

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Help", |ui| if ui.button("About").clicked() {});
            });
        });

        egui::SidePanel::left("controls")
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_width(350.0);
                ui.style_mut().spacing.slider_width = 200.0;

                egui::TopBottomPanel::bottom("controls_footer")
                    .exact_height(70.0)
                    .show_inside(ui, |ui| {
                        ui.add_space(10.0);
                        let width = ui.clip_rect().width() * 0.46;

                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("Reset Source").color(
                                        if self.dirty_source_config && self.flash_state {
                                            Color32::ORANGE
                                        } else {
                                            Color32::WHITE
                                        },
                                    ))
                                    .min_size(Vec2::new(width, 0.0)),
                                )
                                .clicked()
                            {
                                self.state.lock().unwrap().reset_source = true;
                                self.dirty_source_config = false;
                            }

                            if ui
                                .add(
                                    egui::Button::new(RichText::new("Reset Device").color(
                                        if self.dirty_device_config && self.flash_state {
                                            Color32::ORANGE
                                        } else {
                                            Color32::WHITE
                                        },
                                    ))
                                    .min_size(Vec2::new(width, 0.0)),
                                )
                                .clicked()
                            {
                                self.state.lock().unwrap().reset_device = true;
                                self.dirty_device_config = false;
                            }
                        });

                        ui.add_space(5.0);

                        ui.horizontal(|ui| {
                            if ui
                                .add(egui::Button::new("Save").min_size(Vec2::new(width, 0.0)))
                                .clicked()
                            {
                                // save
                                should_save = true;
                            }

                            if ui
                                .add(egui::Button::new("Load...").min_size(Vec2::new(width, 0.0)))
                                .clicked()
                            {
                                match native_dialog::FileDialogBuilder::default()
                                    .set_location(&save_dir())
                                    .open_single_file()
                                    .show()
                                {
                                    Ok(result) => load_path = result,
                                    Err(err) => error!("Could not pick config file path: {err}"),
                                }
                            }
                        });
                    });

                ui.heading("Control Panel");
                ui.separator();

                // hack to prevent text clipping through the footer bar
                ui.shrink_clip_rect(Rect {
                    min: Pos2 {
                        x: f32::NEG_INFINITY,
                        y: 0.0,
                    },
                    max: Pos2 {
                        x: f32::INFINITY,
                        y: ui.clip_rect().bottom() - 45.0,
                    },
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let old_update_frequency = config.update_frequency;
                    egui::ComboBox::new("update_freq", "Update Frequency")
                        .selected_text(format!("{} Hz", config.update_frequency))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut config.update_frequency, 5, "5 Hz");
                            ui.selectable_value(&mut config.update_frequency, 30, "30 Hz");
                            ui.selectable_value(&mut config.update_frequency, 50, "50 Hz");
                            ui.selectable_value(&mut config.update_frequency, 60, "60 Hz");
                            ui.selectable_value(&mut config.update_frequency, 100, "100 Hz");
                            ui.selectable_value(&mut config.update_frequency, 125, "125 Hz");
                            ui.selectable_value(&mut config.update_frequency, 500, "500 Hz");
                            ui.selectable_value(&mut config.update_frequency, 1000, "1000 Hz");
                        });
                    dirty_config |= config.update_frequency != old_update_frequency;

                    ui.separator();
                    ui.heading("Steering Wheel");
                    dirty_config |= ui
                        .add(
                            egui::Slider::new(&mut config.range, 30.0..=1800.0)
                                .step_by(30.0)
                                .custom_formatter(|v, _| format!("±{v:.0}°"))
                                .text("Range"),
                        )
                        .changed();

                    dirty_config |= ui
                        .add(
                            egui::Slider::new(&mut config.horn_radius, 0.1..=1.0)
                                .step_by(0.1)
                                .text("Horn Radius"),
                        )
                        .changed();

                    dirty_config |= ui
                        .add(
                            egui::Slider::new(&mut config.base_radius, 0.0..=1.0)
                                .step_by(0.1)
                                .text("Base Radius"),
                        )
                        .changed();

                    ui.horizontal(|ui| {
                        dirty_config |= ui
                            .add(
                                egui::DragValue::new(&mut config.inertia)
                                    .speed(0.1)
                                    .range(0.1..=1000.0)
                                    .clamp_existing_to_range(true),
                            )
                            .changed();
                        ui.label("Inertia (kg*m^2)");
                    });

                    ui.horizontal(|ui| {
                        dirty_config |= ui
                            .add(
                                egui::DragValue::new(&mut config.friction)
                                    .speed(0.5)
                                    .range(0.0..=100.0)
                                    .clamp_existing_to_range(true),
                            )
                            .changed();
                        ui.label("Friction");
                    });

                    ui.horizontal(|ui| {
                        dirty_config |= ui
                            .add(
                                egui::DragValue::new(&mut config.spring)
                                    .speed(0.5)
                                    .range(0.0..=100.0)
                                    .clamp_existing_to_range(true),
                            )
                            .changed();
                        ui.label("Spring");
                    });

                    ui.horizontal(|ui| {
                        dirty_config |= ui
                            .add(
                                egui::DragValue::new(&mut config.max_torque)
                                    .speed(0.1)
                                    .range(-1000.0..=1000.0)
                                    .clamp_existing_to_range(true),
                            )
                            .changed();
                        ui.label("Max Torque (Nm)");
                    });

                    ui.separator();
                    dirty_wheel |= ui
                        .add(
                            egui::Slider::new(
                                &mut wheel.angle,
                                -(config.range * 0.5)..=(config.range * 0.5),
                            )
                            .drag_value_speed(1.0)
                            .custom_formatter(|v, _| format!("{v:.1}°"))
                            .text("Angle"),
                        )
                        .changed();

                    ui.separator();
                    ui.heading("Input");

                    ui.horizontal(|ui| {
                        dirty_config |= ui
                            .add(
                                egui::DragValue::new(&mut config.pressure_threshold)
                                    .speed(1)
                                    .range(0..=2048)
                                    .clamp_existing_to_range(true),
                            )
                            .changed();
                        ui.label("Pen Pressure Threshold");
                    });

                    let old_source = config.source;
                    egui::ComboBox::new("source", "Input Source")
                        .selected_text(old_source.to_string())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut config.source,
                                config::Source::None,
                                "Disabled",
                            );
                            ui.selectable_value(
                                &mut config.source,
                                config::Source::Net,
                                "Network (over UDP)",
                            );
                            #[cfg(target_os = "windows")]
                            ui.selectable_value(
                                &mut config.source,
                                config::Source::Wintab,
                                "Wacom Wintab (Windows)",
                            );
                            #[cfg(target_os = "linux")]
                            ui.selectable_value(
                                &mut config.source,
                                config::Source::Evdev,
                                "Evdev (Linux)",
                            );
                        });

                    if config.source != old_source {
                        dirty_config = true;
                        self.dirty_source_config = true;
                    }

                    match old_source {
                        config::Source::None => {
                            ui.colored_label(Color32::YELLOW, "No input available!");
                        }
                        config::Source::Net => {
                            ui.horizontal(|ui| {
                                ui.label("Listen to: ");
                                dirty_config |=
                                    ui.text_edit_singleline(&mut config.net_sock_addr).changed();
                            });
                        }
                        #[cfg(target_os = "windows")]
                        config::Source::Wintab => {
                            ui.colored_label(Color32::YELLOW, "Work in progress...");
                        }
                        #[cfg(target_os = "linux")]
                        config::Source::Evdev => {
                            ui.heading("Evdev:");
                            let mut changed = false;
                            egui::ComboBox::new("tablet_pref", "Preferred Tablet")
                                .width(200.0)
                                .selected_text(if let Some(dev) = &config.preferred_tablet {
                                    dev.as_str()
                                } else {
                                    "Automatic"
                                })
                                .show_ui(ui, |ui| {
                                    changed |= ui
                                        .selectable_value(
                                            &mut config.preferred_tablet,
                                            None,
                                            "Automatic",
                                        )
                                        .clicked();

                                    if let Some(devices) = &self.evdev_available_devices {
                                        for dev in devices {
                                            changed |= ui
                                                .selectable_value(
                                                    &mut config.preferred_tablet,
                                                    Some(dev.clone()),
                                                    dev,
                                                )
                                                .clicked();
                                        }
                                    } else {
                                        use crate::source::evdev;
                                        match evdev::enumerate_available_devices() {
                                            Ok(devs) => self.evdev_available_devices = Some(devs),
                                            Err(err) => error!("Device enumeration error: {err}"),
                                        }
                                    }
                                });

                            if changed {
                                dirty_config = true;
                                self.dirty_device_config = true;
                                self.flash_cooldown = 0.0;
                            }
                        }
                    }

                    ui.separator();
                    ui.heading("Mapping");
                    ui.colored_label(Color32::YELLOW, "Work in progress...");

                    ui.separator();
                    ui.heading("Output");

                    let old_device = config.device;
                    egui::ComboBox::new("device", "Output Device")
                        .selected_text(old_device.to_string())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut config.device, config::Device::None, "Null");
                            #[cfg(target_os = "linux")]
                            ui.selectable_value(
                                &mut config.device,
                                config::Device::UInput,
                                "Linux uinput",
                            );
                            #[cfg(target_os = "windows")]
                            ui.selectable_value(
                                &mut config.device,
                                config::Device::VigemBus,
                                "ViGEm Bus",
                            );
                        });

                    if config.device != old_device {
                        dirty_config = true;
                        self.dirty_device_config = true;
                        self.flash_cooldown = 0.0;
                    }

                    match old_device {
                        config::Device::None => {
                            ui.colored_label(Color32::YELLOW, "No output available!");
                        }
                        #[cfg(target_os = "linux")]
                        config::Device::UInput => {
                            ui.heading("Virtual Controller: (via uinput)");
                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                if ui.text_edit_singleline(&mut config.device_name).changed() {
                                    dirty_config = true;
                                    self.dirty_device_config = true;
                                }
                            });
                            ui.monospace(format!("vendor = 0x{:x}", config.device_vendor));
                            ui.monospace(format!("product = 0x{:x}", config.device_product));
                            ui.monospace(format!("version = 0x{:x}", config.device_version));
                        }
                        #[cfg(target_os = "windows")]
                        config::Device::VigemBus => {
                            ui.colored_label(Color32::YELLOW, "Work in progress...");
                        }
                    }
                });
            });

        egui::TopBottomPanel::bottom("steer_bar")
            .exact_height(32.0)
            .show(ctx, |ui| {
                let ui_rect = ui.min_rect();

                let centre = ui_rect.center().x;
                let bound = ui_rect.width() * 0.5;
                let range = config.range * 0.5;
                let mut min = 0.0;
                let mut max = (wheel.angle / range) * bound;
                let mut colour = Color32::BLUE;

                if min > max {
                    std::mem::swap(&mut min, &mut max);
                    colour = Color32::RED;
                }

                let bar_rect = Rect {
                    min: Pos2::new(centre + min, ui_rect.min.y),
                    max: Pos2::new(centre + max, ui_rect.max.y),
                };

                ui.painter_at(ui_rect)
                    .rect_filled(bar_rect, CornerRadius::ZERO, colour);

                // allow user to click on the bar to set the angle
                if let Some(pos) = ui
                    .interact(ui_rect, Id::new("steer_bar_click"), Sense::click_and_drag())
                    .hover_pos()
                {
                    let left = ui_rect.left();
                    let right = ui_rect.right();

                    if pos.x >= left && pos.x <= right && ui.input(|i| i.pointer.any_down()) {
                        wheel.angle = remap(pos.x, left, right, -range, range);

                        dirty_wheel = true;
                    }
                }
            });

        egui::TopBottomPanel::bottom("ff_bar")
            .exact_height(16.0)
            .show(ctx, |ui| {
                let ui_rect = ui.min_rect();

                let centre = ui_rect.center().x;
                let bound = ui_rect.width() * 0.5;
                let mut min = 0.0;
                let mut max = (wheel.feedback_torque / config.max_torque) * bound;
                let colour = Color32::BROWN;

                if min > max {
                    std::mem::swap(&mut min, &mut max);
                }

                let bar_rect = Rect {
                    min: Pos2::new(centre + min, ui_rect.min.y),
                    max: Pos2::new(centre + max, ui_rect.max.y),
                };

                ui.painter_at(ui_rect)
                    .rect_filled(bar_rect, CornerRadius::ZERO, colour);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // draw the (somewhat primitive) steering wheel

            let colour = Color32::LIGHT_GRAY;
            let pen_colour = Color32::MAGENTA;
            let horn_colour = Color32::PURPLE;

            let mut rect = ctx.available_rect().scale_from_center(0.95);

            // keep the rect a square
            if rect.width() > rect.height() {
                let extra = rect.width() - rect.height();
                rect = rect.shrink2(Vec2::X * extra * 0.5);
            } else if rect.height() > rect.width() {
                let extra = rect.height() - rect.width();
                rect = rect.shrink2(Vec2::Y * extra * 0.5);
            }

            let origin = rect.center();
            let size = rect.size().x.min(rect.size().y) * 0.45;
            let stroke = Stroke::new(size * 0.1, colour);
            let painter = ui.painter_at(ctx.available_rect());

            let sin = wheel.angle.to_radians().sin();
            let cos = wheel.angle.to_radians().cos();
            let rightward = Vec2::new(size * cos, size * sin);
            let downward = Vec2::new(-size * sin, size * cos);

            let left = rect.left();
            let right = rect.right();
            let bottom = rect.bottom();
            let top = rect.top();

            painter.circle_stroke(origin, size, stroke);
            painter.line_segment([origin + rightward, origin - rightward], stroke);
            painter.line_segment([origin, origin + downward], stroke);
            painter.circle_filled(
                origin,
                size * config.horn_radius,
                if wheel.honking { horn_colour } else { colour },
            );

            if let Some(pen) = pen {
                let pos = Pos2 {
                    x: remap(pen.x, -1.0, 1.0, right, left),
                    y: remap(pen.y, -1.0, 1.0, top, bottom),
                };

                if pen.pressure > config.pressure_threshold {
                    painter.circle_filled(pos, 5.0, pen_colour);
                } else {
                    painter.circle_stroke(pos, 5.0, Stroke::new(1.0, pen_colour));
                }
            }

            // allow user to click and drag the steering wheel
            if let Some(pos) = ui
                .interact(rect, Id::new("wheel_box"), Sense::click_and_drag())
                .hover_pos()
            {
                if rect.contains(pos) && ui.input(|i| i.pointer.primary_down()) {
                    let x = remap(pos.x, right, left, -1.0, 1.0);
                    let y = remap(pos.y, top, bottom, -1.0, 1.0);

                    pen_override = Some(Pen {
                        x,
                        y,
                        pressure: u32::MAX,
                        ..Default::default()
                    });
                }
            }
        });

        let mut state2 = self.state.lock().unwrap();

        if dirty_config {
            state2.config = config.clone();
        }

        if dirty_wheel {
            state2.wheel.angle = wheel.angle;
        }

        state2.pen_override = pen_override.clone();

        // prevent double lock with the save/load code below
        drop(state2);

        if should_save {
            let path = save_path();
            debug!("Saving configuration to {}", path.display());
            if let Err(err) = save_file(&config, &path) {
                self.state.lock().unwrap().last_error =
                    Some(err.context("Could not save configuration file."));
            }
        }

        if let Some(path) = load_path {
            debug!("Loading configuration at {}", path.display());
            let mut new_config = Config::default();
            match load_file(&mut new_config, &path) {
                Ok(parse_errors) => {
                    if !parse_errors.is_empty() {
                        self.state.lock().unwrap().last_error =
                            Some(anyhow!(compile_parse_errors(parse_errors)));
                    }

                    let mut state2 = self.state.lock().unwrap();
                    state2.config = new_config;
                    state2.reset_device = true;
                    state2.reset_source = true;
                }
                Err(load_err) => {
                    self.state.lock().unwrap().last_error =
                        Some(load_err.context("Could not load configuration file."));
                }
            }
        }
    }
}

pub fn gui(state: Arc<Mutex<State>>) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder {
            title: Some("Pen Steer".into()),
            app_id: Some("pen-steer".into()),
            ..Default::default()
        },
        persist_window: false,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "pen-steer",
        options,
        Box::new(|cc| Ok(Box::new(GuiApp::new(state, cc)))),
    )
}

fn remap(t: f32, a1: f32, a2: f32, b1: f32, b2: f32) -> f32 {
    b1 + (t - a1) * (b2 - b1) / (a2 - a1)
}
