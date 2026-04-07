use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime},
};

use crate::{
    config::{self, Config},
    controller::{self, Command, Event, Snapshot},
    mapping::MapOrientation,
    math,
    pen::Pen,
    save::{compile_parse_errors, load_file, save_file},
    save_path::{save_dir, save_path},
    util,
    wheel::Wheel,
};
use anyhow::anyhow;
use eframe::egui::{
    self, Color32, Context, CornerRadius, Frame, Id, Layout, OpenUrl, Pos2, Rect, RichText, Sense,
    Stroke, Ui, Vec2, ViewportBuilder,
};
use log::{debug, error, info, warn};
use triple_buffer::triple_buffer;

#[derive(Clone, Copy)]
enum SaveAction {
    None,
    ToCurrentPath,
    ToCustomPath,
}

pub struct GuiApp {
    cmd_tx: rtrb::Producer<Command>,
    event_rx: rtrb::Consumer<Event>,
    snapshot_rx: triple_buffer::Output<Snapshot>,
    config: Config,
    config_dirty: bool,
    snapshot: Snapshot,
    pen_override: Option<Pen>,
    last_error: Option<anyhow::Error>,
    quit_flag: Arc<AtomicBool>,
    save_path: PathBuf,
    evdev_available_devices: Option<Vec<String>>,
    dirty_source_config: bool,
    dirty_device_config: bool,
    save_action: SaveAction,
    should_load: bool,
    show_wheel: bool,
    show_about: bool,
    device_vendor_edit_buf: String,
    device_product_edit_buf: String,
    device_version_edit_buf: String,
    base_radius_selection: Option<f32>,
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.quit_flag.load(Ordering::Acquire) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        draw_about(ctx, &mut self.show_about);

        self.save();
        self.load();
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 1.0, 0.0, 0.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if let Some(err) = self.last_error.take() {
            show_error(frame, &err);
        }

        if let Ok(Event::Error(err)) = self.event_rx.pop() {
            show_error(frame, &err);
        }

        self.snapshot = self.snapshot_rx.read().clone();

        self.draw_ui(ui);

        if self.config_dirty {
            self.config_dirty = false;

            let _ = self.cmd_tx.push(Command::UpdateConfig {
                new_config: self.config.clone(),
            });
        }

        if self.show_wheel {
            let cooldown = Duration::from_secs_f64(f64::from(self.config.update_frequency).recip());
            ui.request_repaint_after(cooldown);
        }
    }
}

impl GuiApp {
    pub fn new(quit_flag: Arc<AtomicBool>) -> Self {
        let save_path = save_path();
        let show_about = !save_path.exists();
        let mut last_error = None;

        info!("Loading configuration file: {}", save_path.display());
        let mut config = Config::default();
        match load_file(&mut config, &save_path) {
            Ok(parse_errors) => {
                if !parse_errors.is_empty() {
                    last_error = Some(anyhow!(compile_parse_errors(parse_errors)));
                }
            }
            Err(load_err) => {
                if let Some(err) = load_err.downcast_ref::<std::io::Error>()
                    && let std::io::ErrorKind::NotFound = err.kind()
                {
                    warn!("No configuration file present. Using defaults...");
                } else {
                    last_error = Some(load_err.context("Could not load configuration file."));
                }
            }
        }
        info!("Configuration loaded.");

        let (cmd_tx, cmd_rx) = rtrb::RingBuffer::new(4);
        let (event_tx, event_rx) = rtrb::RingBuffer::new(4);
        let (snapshot_tx, snapshot_rx) = triple_buffer(&Snapshot::default());

        let config_cloned = config.clone();
        let quit_flag_cloned = quit_flag.clone();
        std::thread::spawn(move || {
            controller::controller(
                config_cloned,
                cmd_rx,
                event_tx,
                snapshot_tx,
                quit_flag_cloned,
            );
        });

        Self {
            cmd_tx,
            event_rx,
            snapshot_rx,
            snapshot: Snapshot::default(),
            config,
            config_dirty: false,
            pen_override: None,
            last_error,
            quit_flag,
            save_path,
            evdev_available_devices: None,
            dirty_source_config: false,
            dirty_device_config: false,
            save_action: SaveAction::None,
            should_load: false,
            show_wheel: true,
            show_about,
            device_vendor_edit_buf: String::new(),
            device_product_edit_buf: String::new(),
            device_version_edit_buf: String::new(),
            base_radius_selection: None,
        }
    }

    fn save(&mut self) {
        let action = self.save_action;
        self.save_action = SaveAction::None;

        let path = match action {
            SaveAction::None => {
                return;
            }
            SaveAction::ToCurrentPath => self.save_path.clone(),
            SaveAction::ToCustomPath => {
                match native_dialog::FileDialogBuilder::default()
                    .set_location(&save_dir())
                    .save_single_file()
                    .show()
                {
                    Ok(Some(path)) => path,
                    Ok(None) => return,
                    Err(err) => {
                        error!("Could not pick config file save path: {err}");
                        return;
                    }
                }
            }
        };

        debug!("Saving configuration to {}", path.display());
        if let Err(err) = save_file(&self.config, &path) {
            self.last_error = Some(err.context("Could not save configuration file."));
        }
    }

    fn load(&mut self) {
        if !self.should_load {
            return;
        }

        self.should_load = false;

        let path = match native_dialog::FileDialogBuilder::default()
            .set_location(&save_dir())
            .open_single_file()
            .show()
        {
            Ok(Some(path)) => path,
            Ok(None) => return,
            Err(err) => {
                error!("Could not pick config file path: {err}");
                return;
            }
        };

        debug!("Loading configuration at {}", path.display());
        let mut config = Config::default();
        let parse_errors = match load_file(&mut config, &path) {
            Ok(parse_errors) => parse_errors,
            Err(load_err) => {
                self.last_error = Some(load_err.context("Could not load configuration file."));
                return;
            }
        };

        if !parse_errors.is_empty() {
            self.last_error = Some(anyhow!(compile_parse_errors(parse_errors)));
        }

        let _ = self.cmd_tx.push(Command::Initialise { new_config: config });

        self.device_vendor_edit_buf.clear();
        self.device_product_edit_buf.clear();
        self.device_version_edit_buf.clear();
    }
}

fn show_error(frame: &eframe::Frame, err: &anyhow::Error) {
    error!("\n* * * * * * * * * *\n{err:?}\n* * * * * * * * * *");

    let _ = native_dialog::MessageDialogBuilder::default()
        .set_level(native_dialog::MessageLevel::Error)
        .set_title("Pen Steer: Controller Error")
        .set_owner(frame)
        .set_text(format!("{err:?}"))
        .alert()
        .show();
}

impl GuiApp {
    fn draw_menu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Save").clicked() {
                    self.save_action = SaveAction::ToCurrentPath;
                }

                if ui.button("Save as...").clicked() {
                    self.save_action = SaveAction::ToCustomPath;
                }

                self.should_load |= ui.button("Load...").clicked();

                ui.separator();
                if ui.button("Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("About").clicked() {
                    self.show_about = true;
                }
            });

            ui.with_layout(Layout::right_to_left(egui::Align::Max), |ui| {
                let string = if self.show_wheel {
                    "Hide wheel"
                } else {
                    "Show wheel"
                };
                if ui.button(string).clicked() {
                    self.show_wheel = !self.show_wheel;
                }
            });
        });
    }

    fn draw_ui(&mut self, ui: &mut Ui) {
        egui::Panel::top("menu").show_inside(ui, |ui| self.draw_menu(ui));

        egui::Panel::left("controls")
            .resizable(false)
            .show_inside(ui, |ui| {
                const FOOTER_HEIGHT: f32 = 70.0;

                ui.set_width(350.0);
                ui.style_mut().spacing.slider_width = 200.0;

                egui::Panel::bottom("controls_footer")
                    .exact_size(FOOTER_HEIGHT)
                    .show_inside(ui, |ui| {
                        self.draw_controls_footer(ui);
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
                        y: ui.clip_rect().bottom() - FOOTER_HEIGHT - 4.0,
                    },
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.draw_controls(ui);
                });
            });

        if !self.show_wheel {
            self.draw_steering_wheel_placeholder(ui);
            return;
        }

        egui::Panel::bottom("steer_bar")
            .frame(Frame {
                fill: Color32::TRANSPARENT,
                ..Default::default()
            })
            .exact_size(32.0)
            .show_inside(ui, |ui| {
                if let Some(new_angle) = draw_steer_bar(self.snapshot.wheel.angle, &self.config, ui)
                {
                    self.snapshot.wheel.angle = new_angle;

                    let _ = self.cmd_tx.push(Command::TeleportWheel {
                        new_wheel: self.snapshot.wheel.clone(),
                    });
                }
            });

        if self.snapshot.feedback.is_some() {
            egui::Panel::bottom("ff_bar")
                .frame(Frame {
                    fill: Color32::TRANSPARENT,
                    ..Default::default()
                })
                .exact_size(16.0)
                .show_inside(ui, |ui| {
                    draw_ff_bar(
                        self.snapshot.wheel.feedback_torque,
                        self.config.max_torque,
                        ui,
                    );
                });
        }

        egui::CentralPanel::default()
            .frame(Frame {
                fill: Color32::TRANSPARENT,
                ..Default::default()
            })
            .show_inside(ui, |ui| {
                let style = ui.style_mut();
                style.visuals.panel_fill = Color32::TRANSPARENT;
                style.visuals.window_fill = Color32::TRANSPARENT;

                let pen_override = draw_steering_wheel(
                    &self.config,
                    &self.snapshot.wheel,
                    self.base_radius_selection,
                    self.snapshot.pen,
                    ui,
                );

                if self.pen_override != pen_override {
                    let _ = self.cmd_tx.push(Command::SetPenOverride { pen: pen_override });
                    self.pen_override = pen_override;
                }
            });
    }

    fn draw_controls_footer(&mut self, ui: &mut Ui) {
        ui.add_space(10.0);
        let width = ui.clip_rect().width() * 0.46;

        let source_btn = egui::Button::new(RichText::new("Reset Source").color(
            if self.dirty_source_config {
                Color32::ORANGE
            } else {
                Color32::WHITE
            },
        ))
        .min_size(Vec2::new(width, 0.0));

        let device_btn = egui::Button::new(RichText::new("Reset Device").color(
            if self.dirty_device_config {
                Color32::ORANGE
            } else {
                Color32::WHITE
            },
        ))
        .min_size(Vec2::new(width, 0.0));

        ui.horizontal(|ui| {
            if ui.add(source_btn).clicked() {
                let _ = self.cmd_tx.push(Command::ResetSource);
                self.dirty_source_config = false;
            }

            if ui.add(device_btn).clicked() {
                let _ = self.cmd_tx.push(Command::ResetDevice);
                self.dirty_device_config = false;
            }
        });

        ui.add_space(5.0);

        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new("Save").min_size(Vec2::new(width, 0.0)))
                .clicked()
            {
                self.save_action = SaveAction::ToCurrentPath;
            }

            self.should_load |= ui
                .add(egui::Button::new("Load...").min_size(Vec2::new(width, 0.0)))
                .clicked();
        });
    }

    fn draw_controls(&mut self, ui: &mut Ui) {
        const BASE_RADIUS_TOOLTIP: &str = "Minimum radius for angular \
        displacement calculations.\nCircling the pen closer than this radius \
        will not cause the wheel to spin faster.\n\n\
        This can prevent issues when making off-centred circles, but if the \
        pen is consistently too close, it will cause the wheel to turn slower \
        than intended.";

        let dirty = &mut self.config_dirty;

        egui::ComboBox::new("update_freq", "Update Frequency")
            .selected_text(format!("{} Hz", self.config.update_frequency))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.config.update_frequency, 5, "5 Hz")
                    .mark(dirty);
                ui.selectable_value(&mut self.config.update_frequency, 30, "30 Hz")
                    .mark(dirty);
                ui.selectable_value(&mut self.config.update_frequency, 50, "50 Hz")
                    .mark(dirty);
                ui.selectable_value(&mut self.config.update_frequency, 60, "60 Hz")
                    .mark(dirty);
                ui.selectable_value(&mut self.config.update_frequency, 100, "100 Hz")
                    .mark(dirty);
                ui.selectable_value(&mut self.config.update_frequency, 125, "125 Hz")
                    .mark(dirty);
                ui.selectable_value(&mut self.config.update_frequency, 500, "500 Hz")
                    .mark(dirty);
                ui.selectable_value(&mut self.config.update_frequency, 1000, "1000 Hz")
                    .mark(dirty);
            });

        ui.separator();
        ui.style_mut().spacing.interact_size.x = 60.0;
        ui.heading("Steering Wheel");
        ui.add(
            egui::Slider::new(&mut self.config.range, 30.0..=1800.0)
                .step_by(30.0)
                .custom_formatter(|v, _| format!("±{v:.0}°"))
                .text("Range"),
        )
        .mark(dirty);

        ui.add(
            egui::Slider::new(&mut self.config.horn_radius, 0.1..=1.0)
                .step_by(0.1)
                .text("Horn Radius"),
        )
        .mark(dirty);

        let base_radius_response = ui
            .add(
                egui::Slider::new(&mut self.config.base_radius, 0.0..=1.0)
                    .step_by(0.1)
                    .text("Base Radius"),
            )
            .mark(dirty);

        let base_radius_changing = base_radius_response.dragged() || base_radius_response.hovered();
        self.base_radius_selection = base_radius_changing.then_some(self.config.base_radius);

        base_radius_response.on_hover_text(BASE_RADIUS_TOOLTIP);

        ui.style_mut().spacing.interact_size.x = 150.0;

        ui.horizontal(|ui| {
            ui.label("Inertia: ");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    egui::DragValue::new(&mut self.config.inertia)
                        .speed(0.1)
                        .range(0.01..=f32::MAX)
                        .suffix(" kg×m²"),
                )
                .mark(dirty);
            });
        });

        ui.horizontal(|ui| {
            ui.label("Friction coefficient: ");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    egui::DragValue::new(&mut self.config.friction)
                        .speed(0.5)
                        .range(0.0..=f32::MAX)
                        .suffix(" Nm/rad/s"),
                )
                .mark(dirty);
            });
        });

        ui.horizontal(|ui| {
            ui.label("Spring stiffness:");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    egui::DragValue::new(&mut self.config.spring)
                        .speed(0.5)
                        .range(0.0..=f32::MAX)
                        .suffix(" Nm/rad"),
                )
                .mark(dirty);
            });
        });

        ui.horizontal(|ui| {
            ui.label("Max feedback torque: ");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    egui::DragValue::new(&mut self.config.max_torque)
                        .speed(0.1)
                        .range(f32::MIN..=f32::MAX)
                        .suffix(" Nm"),
                )
                .mark(dirty);
            });
        });

        if self.show_wheel {
            let half_range = self.config.half_range_rad();

            ui.separator();
            ui.style_mut().spacing.interact_size.x = 40.0;
            if ui
                .add(
                    egui::Slider::new(&mut self.snapshot.wheel.angle, -half_range..=half_range)
                        .drag_value_speed(1.0f64.to_radians())
                        .custom_formatter(|v, _| format!("{:.1}°", v.to_degrees()))
                        .text("Angle"),
                )
                .changed()
            {
                let _ = self.cmd_tx.push(Command::TeleportWheel {
                    new_wheel: self.snapshot.wheel.clone(),
                });
            }
        }

        ui.separator();
        ui.heading("Input");

        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.config.pressure_threshold)
                    .speed(1)
                    .range(0..=2048)
                    .clamp_existing_to_range(true),
            )
            .mark(dirty);
            ui.label("Pen Pressure Threshold");
        });

        let old_source = self.config.source;
        egui::ComboBox::new("source", "Input Source")
            .selected_text(old_source.to_string())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.config.source, config::Source::None, "Disabled");
                ui.selectable_value(
                    &mut self.config.source,
                    config::Source::Net,
                    "Network (over UDP)",
                )
                .mark(dirty);
                #[cfg(target_os = "windows")]
                ui.selectable_value(
                    &mut self.config.source,
                    config::Source::Wintab,
                    "Wacom Wintab (Windows)",
                )
                .mark(dirty);
                #[cfg(target_os = "linux")]
                ui.selectable_value(
                    &mut self.config.source,
                    config::Source::Evdev,
                    "Evdev (Linux)",
                )
                .mark(dirty);
            });

        self.dirty_source_config |= self.config.source != old_source;

        match old_source {
            config::Source::None => {
                ui.colored_label(Color32::YELLOW, "No input available!");
            }
            config::Source::Net => {
                ui.horizontal(|ui| {
                    ui.label("Listen to: ");
                    ui.text_edit_singleline(&mut self.config.net_sock_addr)
                        .mark(dirty);
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
                    .selected_text(if let Some(dev) = &self.config.preferred_tablet {
                        dev.as_str()
                    } else {
                        "Automatic"
                    })
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(&mut self.config.preferred_tablet, None, "Automatic")
                            .clicked();

                        if let Some(devices) = &self.evdev_available_devices {
                            for dev in devices {
                                changed |= ui
                                    .selectable_value(
                                        &mut self.config.preferred_tablet,
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
                    *dirty = true;
                    self.dirty_source_config = true;
                }
            }
        }

        ui.separator();
        ui.heading("Mapping");
        ui.style_mut().spacing.interact_size.x = 65.0;
        let map = &mut self.config.mapping;
        ui.horizontal(|ui| {
            ui.label("Input:");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::DragValue::new(&mut map.min_in_x).speed(0.1))
                    .mark(dirty);
                ui.add(egui::DragValue::new(&mut map.min_in_y).speed(0.1))
                    .mark(dirty);
                ui.add(egui::DragValue::new(&mut map.max_in_x).speed(0.1))
                    .mark(dirty);
                ui.add(egui::DragValue::new(&mut map.max_in_y).speed(0.1))
                    .mark(dirty);
            });
        });
        ui.horizontal(|ui| {
            ui.label("Output:");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::DragValue::new(&mut map.min_out_x).speed(0.1))
                    .mark(dirty);
                ui.add(egui::DragValue::new(&mut map.min_out_y).speed(0.1))
                    .mark(dirty);
                ui.add(egui::DragValue::new(&mut map.max_out_x).speed(0.1))
                    .mark(dirty);
                ui.add(egui::DragValue::new(&mut map.max_out_y).speed(0.1))
                    .mark(dirty);
            });
        });
        egui::ComboBox::new("map-orient", "Orientation")
            .selected_text(format!("{:?}", map.orientation))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut map.orientation, MapOrientation::None, "A0")
                    .mark(dirty);
                ui.selectable_value(&mut map.orientation, MapOrientation::A90, "A90")
                    .mark(dirty);
                ui.selectable_value(&mut map.orientation, MapOrientation::A180, "A180")
                    .mark(dirty);
                ui.selectable_value(&mut map.orientation, MapOrientation::A270, "A270")
                    .mark(dirty);
            });
        ui.checkbox(&mut map.invert_x, "Invert X axis").mark(dirty);
        ui.checkbox(&mut map.invert_y, "Invert Y axis").mark(dirty);

        ui.separator();
        ui.heading("Extra Buttons");
        ui.style_mut().spacing.interact_size.x = 65.0;

        ui.separator();
        ui.heading("Output");

        let old_device = self.config.device;
        egui::ComboBox::new("device", "Output Device")
            .selected_text(old_device.to_string())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.config.device, config::Device::None, "Null")
                    .mark(dirty);
                #[cfg(target_os = "linux")]
                ui.selectable_value(
                    &mut self.config.device,
                    config::Device::UInput,
                    "Linux uinput",
                )
                .mark(dirty);
                #[cfg(target_os = "windows")]
                ui.selectable_value(
                    &mut self.config.device,
                    config::Device::VigemBus,
                    "ViGEm Bus",
                )
                .mark(dirty);
            });

        if self.config.device != old_device {
            self.dirty_device_config = true;
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
                    self.dirty_device_config |= ui
                        .text_edit_singleline(&mut self.config.device_name)
                        .mark(dirty)
                        .changed();
                });

                ui.horizontal(|ui| {
                    ui.label("Vendor:");
                    self.dirty_device_config |= edit_u16_hex(
                        ui,
                        &mut self.config.device_vendor,
                        &mut self.device_vendor_edit_buf,
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Product:");
                    self.dirty_device_config |= edit_u16_hex(
                        ui,
                        &mut self.config.device_product,
                        &mut self.device_product_edit_buf,
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Version:");
                    self.dirty_device_config |= edit_u16_hex(
                        ui,
                        &mut self.config.device_version,
                        &mut self.device_version_edit_buf,
                    );
                });
            }
            #[cfg(target_os = "windows")]
            config::Device::VigemBus => {
                ui.colored_label(Color32::YELLOW, "Work in progress...");
            }
        }

        if self.dirty_source_config || self.dirty_device_config {
            *dirty = true;
        }
    }

    fn draw_steering_wheel_placeholder(&mut self, ui: &mut Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.centered_and_justified(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Steering wheel view disabled. Click to enable.")
                                .underline(),
                        )
                        .frame(false),
                    )
                    .clicked()
                {
                    self.show_wheel = true;
                }
            })
        });
    }
}

fn draw_steer_bar(angle: f32, config: &Config, ui: &mut Ui) -> Option<f32> {
    let ui_rect = ui.min_rect();

    let centre = ui_rect.center().x;
    let bound = ui_rect.width() * 0.5;
    let range = config.half_range_rad();
    let mut min = 0.0;
    let mut max = (angle / range) * bound;
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
            return Some(math::remap(pos.x, left, right, -range, range));
        }
    }

    None
}

fn draw_ff_bar(torque: f32, max: f32, ui: &mut Ui) {
    let ui_rect = ui.min_rect();

    let centre = ui_rect.center().x;
    let bound = ui_rect.width() * 0.5;
    let mut min = 0.0;
    let mut max = (torque / max) * bound;
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
}

fn draw_steering_wheel(
    config: &Config,
    wheel: &Wheel,
    base_radius_selection: Option<f32>,
    pen: Option<Pen>,
    ui: &mut Ui,
) -> Option<Pen> {
    const BASE_RADIUS_HIGHLIGHT_COLOUR: Color32 =
        Color32::from_rgba_premultiplied(0xAD, 0xD8, 0xE6, 0x80);
    const PEN_COLOUR: Color32 = Color32::CYAN;
    const HORN_COLOUR: Color32 = Color32::PURPLE;
    const PEN_SIZE: f32 = 12.0;
    const HORN_PRESS_SCALE: f32 = 0.9;

    let available_rect = ui.clip_rect();
    let mut rect = available_rect.scale_from_center(0.95);

    // keep the rect a square
    if rect.width() > rect.height() {
        let extra = rect.width() - rect.height();
        rect = rect.shrink2(Vec2::X * extra * 0.5);
    } else if rect.height() > rect.width() {
        let extra = rect.height() - rect.width();
        rect = rect.shrink2(Vec2::Y * extra * 0.5);
    }

    let left = rect.left();
    let right = rect.right();
    let bottom = rect.bottom();
    let top = rect.top();

    let horn_rect = rect.scale_from_center(if wheel.honking {
        config.horn_radius * HORN_PRESS_SCALE
    } else {
        config.horn_radius
    });

    egui::Image::new(egui::include_image!("../resources/base.svg"))
        .alt_text("Base Image")
        .rotate(wheel.angle, Vec2::splat(0.5))
        .paint_at(ui, rect);

    egui::Image::new(egui::include_image!("../resources/inner.svg"))
        .alt_text("Inner Image")
        .rotate(wheel.angle, Vec2::splat(0.5))
        .tint(if wheel.honking {
            HORN_COLOUR
        } else {
            Color32::WHITE
        })
        .paint_at(ui, horn_rect);

    let painter = ui.painter_at(available_rect);

    if let Some(radius) = base_radius_selection {
        painter.circle_filled(
            rect.center(),
            radius * rect.width() * 0.5,
            BASE_RADIUS_HIGHLIGHT_COLOUR,
        );
    }

    if let Some(pen) = pen {
        let pos = Pos2 {
            x: math::remap(pen.x, -1.0, 1.0, right, left),
            y: math::remap(pen.y, -1.0, 1.0, top, bottom),
        };

        if pen.pressure > config.pressure_threshold {
            painter.circle_filled(pos, PEN_SIZE, PEN_COLOUR);
        } else {
            painter.circle_stroke(pos, PEN_SIZE, Stroke::new(2.0, PEN_COLOUR));
        }
    }

    // allow user to click and drag the steering wheel
    if let Some(pos) = ui
        .interact(rect, Id::new("wheel_box"), Sense::click_and_drag())
        .hover_pos()
        && rect.contains(pos)
        && ui.input(|i| i.pointer.primary_down())
    {
        let x = math::remap(pos.x, right, left, -1.0, 1.0);
        let y = math::remap(pos.y, top, bottom, -1.0, 1.0);

        return Some(Pen {
            x,
            y,
            pressure: u32::MAX,
            ..Default::default()
        });
    }

    None
}

fn draw_about(ctx: &Context, show_about: &mut bool) {
    let response = egui::Window::new("barrier_block")
        .open(&mut *show_about)
        .order(egui::Order::Background)
        .title_bar(false)
        .fixed_rect(ctx.viewport_rect())
        .frame(Frame {
            fill: Color32::from_black_alpha(0x80),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.allocate_space(ui.available_size());
        });

    if let Some(response) = response
        && response.response.clicked()
    {
        *show_about = false;
    }

    egui::Window::new("About pen-steer")
        .open(&mut *show_about)
        .collapsible(false)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.heading("Pen Steer");
            ui.small("By JL.");

            ui.separator();
            ui.label(
                "(Ab)Use your graphics tablet as a virtual sim steering wheel. ('cause why not)",
            );
            ui.label("Draw circles to turn the steering wheel!");

            ui.separator();
            if ui.link("Github").clicked() {
                ctx.open_url(OpenUrl::new_tab(
                    "https://github.com/a455jldvmsrwll1a/pen-steer",
                ));
            }
        });

    if ctx.input(|i| i.key_released(egui::Key::Escape)) {
        *show_about = false;
    }
}

pub fn gui() -> eframe::Result {
    let quit_flag = Arc::new(AtomicBool::new(false));
    util::set_handler(quit_flag.clone());

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder {
            title: Some("Pen Steer".into()),
            app_id: Some("pen-steer".into()),
            inner_size: Some(Vec2::new(800.0, 600.0)),
            min_inner_size: Some(Vec2::new(365.0, 0.0)),
            transparent: Some(true),
            ..Default::default()
        },
        persist_window: false,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "pen-steer",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(GuiApp::new(quit_flag)))
        }),
    )
}

fn edit_u16_hex(ui: &mut Ui, value: &mut u16, buf: &mut String) -> bool {
    if buf.is_empty() {
        *buf = format!("0x{value:04X}");
    }

    let out = egui::TextEdit::singleline(buf)
        .char_limit(6)
        .font(egui::TextStyle::Monospace)
        .desired_width(48.0)
        .show(ui);

    let mut dirty = false;
    if out.response.lost_focus() || out.response.clicked_elsewhere() {
        let stripped = buf.trim().trim_start_matches("0x");

        if let Ok(new_value) = u16::from_str_radix(stripped, 16)
            && new_value != *value
        {
            *value = new_value;
            dirty = true;
        }

        buf.clear();
    }

    dirty
}

trait ResponseExt {
    fn mark(self, changed: &mut bool) -> Self;
}

impl ResponseExt for egui::Response {
    fn mark(self, changed: &mut bool) -> Self {
        *changed |= self.changed();
        self
    }
}

impl<T> ResponseExt for egui::InnerResponse<T> {
    fn mark(self, changed: &mut bool) -> Self {
        *changed |= self.response.changed();
        self
    }
}
