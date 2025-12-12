use std::{
    path::PathBuf,
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}},
};

use crate::{
    config::{self, Config}, mapping::MapOrientation, math, pen::Pen, save::{compile_parse_errors, load_file, save_file}, save_path::{save_dir, save_path}, state::State, wheel::Wheel
};
use anyhow::anyhow;
use eframe::egui::{
    self, Color32, Context, CornerRadius, Id, Layout, Pos2, Rect, RichText, Sense, Stroke, Ui,
    Vec2, ViewportBuilder,
};
use log::{debug, error};

#[derive(Clone, Copy)]
enum SaveAction {
    None,
    ToCurrentPath,
    ToCustomPath,
}

pub struct GuiApp {
    state: Arc<Mutex<State>>,
    quit_flag: Arc<AtomicBool>,
    save_path: PathBuf,
    evdev_available_devices: Option<Vec<String>>,
    dirty_source_config: bool,
    dirty_device_config: bool,
    save_action: SaveAction,
    should_load: bool,
    show_wheel: bool,
    device_vendor_edit_buf: String,
    device_product_edit_buf: String,
    device_version_edit_buf: String,
    base_radius_selection: Option<f32>,
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.quit_flag.load(Ordering::Acquire) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        let state_arc = self.state.clone();
        let mut state = state_arc.lock().unwrap();

        if let Some(err) = state.last_error.take() {
            show_error(frame, err);
        }

        self.draw_ui(ctx, &mut state);
        drop(state);

        if self.show_wheel {
            ctx.request_repaint();
        }

        self.save();
        self.load();
    }
}

impl GuiApp {
    pub fn new(state: Arc<Mutex<State>>, quit_flag: Arc<AtomicBool>) -> Self {
        Self {
            state,
            quit_flag,
            save_path: save_path(),
            evdev_available_devices: None,
            dirty_source_config: false,
            dirty_device_config: false,
            save_action: SaveAction::None,
            should_load: false,
            show_wheel: true,
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

        let config = self.state.lock().unwrap().config.clone();
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

        self.device_vendor_edit_buf.clear();
        self.device_product_edit_buf.clear();
        self.device_version_edit_buf.clear();
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

            ui.menu_button("Help", |ui| if ui.button("About").clicked() {});

            ui.with_layout(Layout::right_to_left(egui::Align::Max), |ui| {
                let string = if self.show_wheel { "Hide wheel" } else { "Show wheel" };
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
            self.draw_steering_wheel_placeholder(ctx);
            return;
        }

        egui::TopBottomPanel::bottom("steer_bar")
            .exact_height(32.0)
            .show(ctx, |ui| {
                if let Some(new_angle) = draw_steer_bar(state.wheel.angle, &state.config, ui) {
                    state.wheel.angle = new_angle;
                }
            });

        if let Some(device) = &state.device {
            if device.get_feedback().is_some() {
                egui::TopBottomPanel::bottom("ff_bar")
                    .exact_height(16.0)
                    .show(ctx, |ui| {
                        draw_ff_bar(state.wheel.feedback_torque, state.config.max_torque, ui);
                    });
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let pen = state.pen_override.as_ref().or(state.pen.as_ref());
            state.pen_override = draw_steering_wheel(
                &state.config,
                &state.wheel,
                self.base_radius_selection,
                pen.cloned(),
                ui,
            );
        });
    }

    fn draw_controls_footer(&mut self, ui: &mut Ui, state: &mut State) {
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
            if ui
                .add(egui::Button::new("Save").min_size(Vec2::new(width, 0.0)))
                .clicked()
            {
                self.save_action = SaveAction::ToCurrentPath;
            }

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
        ui.style_mut().spacing.interact_size.x = 60.0;
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

        let base_radius_response = ui.add(
            egui::Slider::new(&mut config.base_radius, 0.0..=1.0)
                .step_by(0.1)
                .text("Base Radius"),
        );

        let base_radius_changing = base_radius_response.dragged() || base_radius_response.hovered();
        self.base_radius_selection = base_radius_changing.then_some(config.base_radius);

        const BASE_RADIUS_TOOLTIP: &str = "Minimum radius for angular \
        displacement calculations.\nCircling the pen closer than this radius \
        will not cause the wheel to spin faster.\n\n\
        This can prevent issues when making off-centred circles, but if the \
        pen is consistently too close, it will cause the wheel to turn slower \
        than intended.";
        base_radius_response.on_hover_text(BASE_RADIUS_TOOLTIP);

        ui.style_mut().spacing.interact_size.x = 150.0;

        ui.horizontal(|ui| {
            ui.label("Inertia: ");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    egui::DragValue::new(&mut config.inertia)
                        .speed(0.1)
                        .range(0.1..=1000.0)
                        .suffix(" kg×m²"),
                );
            });
        });

        ui.horizontal(|ui| {
            ui.label("Friction coefficient: ");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    egui::DragValue::new(&mut config.friction)
                        .speed(0.5)
                        .range(0.0..=100.0),
                );
            });
        });

        ui.horizontal(|ui| {
            ui.label("Spring stiffness:");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    egui::DragValue::new(&mut config.spring)
                        .speed(0.5)
                        .range(0.0..=100.0)
                        .suffix(" Nm/rad"),
                );
            });
        });

        ui.horizontal(|ui| {
            ui.label("Max feedback torque: ");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    egui::DragValue::new(&mut config.max_torque)
                        .speed(0.1)
                        .range(-1000.0..=1000.0)
                        .suffix(" Nm"),
                );
            });
        });

        if self.show_wheel {
            let half_range = config.half_range_rad();

            ui.separator();
            ui.style_mut().spacing.interact_size.x = 40.0;
            ui.add(
                egui::Slider::new(
                    &mut state.wheel.angle,
                    -half_range..=half_range,
                )
                .drag_value_speed(1.0f64.to_radians())
                .custom_formatter(|v, _| format!("{:.1}°", v.to_degrees()))
                .text("Angle"),
            );
        }

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
                }
            }
        }

        ui.separator();
        ui.heading("Mapping");
        ui.style_mut().spacing.interact_size.x = 65.0;
        let map = &mut config.mapping;
        ui.horizontal(|ui| {
            ui.label("Input:");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::DragValue::new(&mut map.min_in_x).speed(0.1));
                ui.add(egui::DragValue::new(&mut map.min_in_y).speed(0.1));
                ui.add(egui::DragValue::new(&mut map.max_in_x).speed(0.1));
                ui.add(egui::DragValue::new(&mut map.max_in_y).speed(0.1));
            });
        });
        ui.horizontal(|ui| {
            ui.label("Output:");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::DragValue::new(&mut map.min_out_x).speed(0.1));
                ui.add(egui::DragValue::new(&mut map.min_out_y).speed(0.1));
                ui.add(egui::DragValue::new(&mut map.max_out_x).speed(0.1));
                ui.add(egui::DragValue::new(&mut map.max_out_y).speed(0.1));
            });
        });
        egui::ComboBox::new("map-orient", "Orientation")
            .selected_text(format!("{:?}", map.orientation))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut map.orientation, MapOrientation::None, "A0");
                ui.selectable_value(&mut map.orientation, MapOrientation::A90, "A90");
                ui.selectable_value(&mut map.orientation, MapOrientation::A180, "A180");
                ui.selectable_value(&mut map.orientation, MapOrientation::A270, "A270");
            });
        ui.checkbox(&mut map.invert_x, "Invert X axis");
        ui.checkbox(&mut map.invert_y, "Invert Y axis");

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

                ui.horizontal(|ui| {
                    ui.label("Vendor:");
                    self.dirty_device_config |= edit_u16_hex(
                        ui,
                        &mut config.device_vendor,
                        &mut self.device_vendor_edit_buf,
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Product:");
                    self.dirty_device_config |= edit_u16_hex(
                        ui,
                        &mut config.device_product,
                        &mut self.device_product_edit_buf,
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Version:");
                    self.dirty_device_config |= edit_u16_hex(
                        ui,
                        &mut config.device_version,
                        &mut self.device_version_edit_buf,
                    );
                });
            }
            #[cfg(target_os = "windows")]
            config::Device::VigemBus => {
                ui.colored_label(Color32::YELLOW, "Work in progress...");
            }
        }
    }

    fn draw_steering_wheel_placeholder(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
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

    let available_rect = ui.ctx().available_rect();
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
    {
        if rect.contains(pos) && ui.input(|i| i.pointer.primary_down()) {
            let x = math::remap(pos.x, right, left, -1.0, 1.0);
            let y = math::remap(pos.y, top, bottom, -1.0, 1.0);

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

pub fn gui(state: Arc<Mutex<State>>, quit_flag: Arc<AtomicBool>) -> eframe::Result {
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
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(GuiApp::new(state, quit_flag)))
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

        if let Ok(new_value) = u16::from_str_radix(stripped, 16) {
            if new_value != *value {
                *value = new_value;
                dirty = true;
            }
        }

        buf.clear();
    }

    dirty
}
