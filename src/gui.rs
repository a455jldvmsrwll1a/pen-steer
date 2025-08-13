use std::sync::{Arc, Mutex};

use crate::{
    config::{self, Config},
    pen::Pen,
    save::{compile_parse_errors, load_file, save_file},
    save_path::{save_dir, save_path},
    state::State,
    wheel::Wheel,
};
use anyhow::anyhow;
use eframe::egui::{
    self, Color32, Context, CornerRadius, Id, Layout, Pos2, Rect, RichText, Sense, Stroke, Ui,
    Vec2, ViewportBuilder,
};
use log::{debug, error};

pub struct GuiApp {
    state: Arc<Mutex<State>>,
    evdev_available_devices: Option<Vec<String>>,
    dirty_source_config: bool,
    dirty_device_config: bool,
    flash_cooldown: f32,
    flash_state: bool,
    should_save: bool,
    should_load: bool,
    show_wheel: bool,
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let state_arc = self.state.clone();
        let mut state = state_arc.lock().unwrap();

        if let Some(err) = state.last_error.take() {
            show_error(frame, err);
        }

        // if wheel is hidden, prevent controller from requesting repaints
        if self.show_wheel && state.gui_context.is_none() {
            state.gui_context = Some(ctx.clone());
        } else if state.gui_context.is_some() {
            state.gui_context = None;
        }

        self.update_flashing_buttons(ctx);
        self.draw_ui(ctx, &mut state);
        drop(state);

        self.save();
        self.load();
    }
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
            should_save: false,
            should_load: false,
            show_wheel: true,
        }
    }

    fn save(&mut self) {
        if !self.should_save {
            return;
        }

        self.should_save = false;

        let config = self.state.lock().unwrap().config.clone();
        let path = save_path();
        debug!("Saving configuration to {}", path.display());
        if let Err(err) = save_file(&config, &path) {
            self.state.lock().unwrap().last_error =
                Some(err.context("Could not save configuration file."));
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
        let mut new_config = Config::default();
        let parse_errors = match load_file(&mut new_config, &path) {
            Ok(parse_errors) => parse_errors,
            Err(load_err) => {
                self.state.lock().unwrap().last_error =
                    Some(load_err.context("Could not load configuration file."));
                return;
            }
        };

        if !parse_errors.is_empty() {
            self.state.lock().unwrap().last_error =
                Some(anyhow!(compile_parse_errors(parse_errors)));
        }

        let mut state2 = self.state.lock().unwrap();
        state2.config = new_config;
        state2.reset_device = true;
        state2.reset_source = true;
    }
}

fn show_error(frame: &eframe::Frame, err: anyhow::Error) {
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
    fn update_flashing_buttons(&mut self, ctx: &Context) {
        self.flash_cooldown -= ctx.input(|i| i.unstable_dt);
        if self.flash_cooldown <= 0.0 {
            self.flash_cooldown = 1.0 / 3.0;
            self.flash_state = !self.flash_state;
        }

        if (self.dirty_source_config || self.dirty_device_config) && !ctx.has_requested_repaint() {
            ctx.request_repaint_after_secs(self.flash_cooldown);
        }
    }

    fn draw_menu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("Help", |ui| if ui.button("About").clicked() {});

            ui.with_layout(Layout::right_to_left(egui::Align::Max), |ui| {
                let string = if self.show_wheel { "<" } else { ">" };
                if ui.button(string).clicked() {
                    self.show_wheel = !self.show_wheel;
                }
            });
        });
    }

    fn draw_ui(&mut self, ctx: &Context, state: &mut State) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| self.draw_menu(ui));

        egui::SidePanel::left("controls")
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_width(350.0);
                ui.style_mut().spacing.slider_width = 200.0;

                const FOOTER_HEIGHT: f32 = 70.0;
                egui::TopBottomPanel::bottom("controls_footer")
                    .exact_height(FOOTER_HEIGHT)
                    .show_inside(ui, |ui| {
                        self.draw_controls_footer(ui, state);
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
                    self.draw_controls(state, ui);
                });
            });

        if !self.show_wheel {
            return;
        }

        egui::TopBottomPanel::bottom("steer_bar")
            .exact_height(32.0)
            .show(ctx, |ui| {
                if let Some(new_angle) = draw_steer_bar(state.wheel.angle, state.config.range, ui) {
                    state.wheel.angle = new_angle;
                }
            });

        egui::TopBottomPanel::bottom("ff_bar")
            .exact_height(16.0)
            .show(ctx, |ui| {
                draw_ff_bar(state.wheel.feedback_torque, state.config.max_torque, ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let pen = state.pen_override.as_ref().or(state.pen.as_ref());
            state.pen_override = draw_steering_wheel(&state.config, &state.wheel, pen.cloned(), ui);
        });
    }

    fn draw_controls_footer(&mut self, ui: &mut Ui, state: &mut State) {
        ui.add_space(10.0);
        let width = ui.clip_rect().width() * 0.46;

        let source_btn = egui::Button::new(RichText::new("Reset Source").color(
            if self.dirty_source_config && self.flash_state {
                Color32::ORANGE
            } else {
                Color32::WHITE
            },
        ))
        .min_size(Vec2::new(width, 0.0));

        let device_btn = egui::Button::new(RichText::new("Reset Device").color(
            if self.dirty_device_config && self.flash_state {
                Color32::ORANGE
            } else {
                Color32::WHITE
            },
        ))
        .min_size(Vec2::new(width, 0.0));

        ui.horizontal(|ui| {
            if ui.add(source_btn).clicked() {
                state.reset_source = true;
                self.dirty_source_config = false;
            }

            if ui.add(device_btn).clicked() {
                state.reset_device = true;
                self.dirty_device_config = false;
            }
        });

        ui.add_space(5.0);

        ui.horizontal(|ui| {
            self.should_save |= ui
                .add(egui::Button::new("Save").min_size(Vec2::new(width, 0.0)))
                .clicked();

            self.should_load |= ui
                .add(egui::Button::new("Load...").min_size(Vec2::new(width, 0.0)))
                .clicked()
        });
    }

    fn draw_controls(&mut self, state: &mut State, ui: &mut Ui) {
        let config = &mut state.config;

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

        ui.separator();
        ui.heading("Steering Wheel");
        ui.add(
            egui::Slider::new(&mut config.range, 30.0..=1800.0)
                .step_by(30.0)
                .custom_formatter(|v, _| format!("±{v:.0}°"))
                .text("Range"),
        );

        ui.add(
            egui::Slider::new(&mut config.horn_radius, 0.1..=1.0)
                .step_by(0.1)
                .text("Horn Radius"),
        );

        ui.add(
            egui::Slider::new(&mut config.base_radius, 0.0..=1.0)
                .step_by(0.1)
                .text("Base Radius"),
        );

        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut config.inertia)
                    .speed(0.1)
                    .range(0.1..=1000.0)
                    .clamp_existing_to_range(true),
            );
            ui.label("Inertia (kg*m^2)");
        });

        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut config.friction)
                    .speed(0.5)
                    .range(0.0..=100.0)
                    .clamp_existing_to_range(true),
            );
            ui.label("Friction");
        });

        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut config.spring)
                    .speed(0.5)
                    .range(0.0..=100.0)
                    .clamp_existing_to_range(true),
            );
            ui.label("Spring");
        });

        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut config.max_torque)
                    .speed(0.1)
                    .range(-1000.0..=1000.0)
                    .clamp_existing_to_range(true),
            );
            ui.label("Max Torque (Nm)");
        });

        ui.separator();
        ui.add(
            egui::Slider::new(
                &mut state.wheel.angle,
                -(config.range * 0.5)..=(config.range * 0.5),
            )
            .drag_value_speed(1.0)
            .custom_formatter(|v, _| format!("{v:.1}°"))
            .text("Angle"),
        );

        ui.separator();
        ui.heading("Input");

        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut config.pressure_threshold)
                    .speed(1)
                    .range(0..=2048)
                    .clamp_existing_to_range(true),
            );
            ui.label("Pen Pressure Threshold");
        });

        let old_source = config.source;
        egui::ComboBox::new("source", "Input Source")
            .selected_text(old_source.to_string())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut config.source, config::Source::None, "Disabled");
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
                ui.selectable_value(&mut config.source, config::Source::Evdev, "Evdev (Linux)");
            });

        self.dirty_source_config |= config.source != old_source;

        match old_source {
            config::Source::None => {
                ui.colored_label(Color32::YELLOW, "No input available!");
            }
            config::Source::Net => {
                ui.horizontal(|ui| {
                    ui.label("Listen to: ");
                    ui.text_edit_singleline(&mut config.net_sock_addr);
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
                            .selectable_value(&mut config.preferred_tablet, None, "Automatic")
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
                    self.dirty_source_config = true;
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
                ui.selectable_value(&mut config.device, config::Device::UInput, "Linux uinput");
                #[cfg(target_os = "windows")]
                ui.selectable_value(&mut config.device, config::Device::VigemBus, "ViGEm Bus");
            });

        if config.device != old_device {
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
                    self.dirty_device_config |=
                        ui.text_edit_singleline(&mut config.device_name).changed();
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
    }
}

fn draw_steer_bar(angle: f32, range: f32, ui: &mut Ui) -> Option<f32> {
    let ui_rect = ui.min_rect();

    let centre = ui_rect.center().x;
    let bound = ui_rect.width() * 0.5;
    let range = range * 0.5;
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
            return Some(remap(pos.x, left, right, -range, range));
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
    pen: Option<Pen>,
    ui: &mut Ui,
) -> Option<Pen> {
    let colour = Color32::LIGHT_GRAY;
    let pen_colour = Color32::MAGENTA;
    let horn_colour = Color32::PURPLE;

    let available_rect = ui.ctx().available_rect();
    let mut rect = available_rect.scale_from_center(0.95);

    let painter = ui.painter_at(available_rect);

    // keep the rect a square
    if rect.width() > rect.height() {
        let extra = rect.width() - rect.height();
        rect = rect.shrink2(Vec2::X * extra * 0.5);
    } else if rect.height() > rect.width() {
        let extra = rect.height() - rect.width();
        rect = rect.shrink2(Vec2::Y * extra * 0.5);
    }

    let size = rect.size().x.min(rect.size().y) * 0.45;
    let stroke = Stroke::new(size * 0.1, colour);

    let sin = wheel.angle.to_radians().sin();
    let cos = wheel.angle.to_radians().cos();
    let rightward = Vec2::new(size * cos, size * sin);
    let downward = Vec2::new(-size * sin, size * cos);

    let left = rect.left();
    let right = rect.right();
    let bottom = rect.bottom();
    let top = rect.top();

    let origin = rect.center();
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

            return Some(Pen {
                x,
                y,
                pressure: u32::MAX,
                ..Default::default()
            });
        }
    }

    None
}

pub fn gui(state: Arc<Mutex<State>>) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder {
            title: Some("Pen Steer".into()),
            app_id: Some("pen-steer".into()),
            inner_size: Some(Vec2::new(800.0, 600.0)),
            min_inner_size: Some(Vec2::new(365.0, 0.0)),
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
