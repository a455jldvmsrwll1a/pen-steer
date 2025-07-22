use std::sync::{Arc, Mutex};

use eframe::egui::{
    self, Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2, ViewportBuilder,
};

use crate::{config, state::State};

pub fn gui(state: Arc<Mutex<State>>) -> eframe::Result {
    let mut config = state.lock().unwrap().config.clone();
    let mut wheel = state.lock().unwrap().wheel.clone();

    let mut dev_started = false;

    let mut dirty_wheel = false;
    let mut dirty_config = false;

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

    eframe::run_simple_native("pen-steer", options, move |ctx, _frame| {
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

        egui::SidePanel::left("controls").show(ctx, |ui| {
            ui.set_width(350.0);
            ui.style_mut().spacing.slider_width = 200.0;

            ui.heading("Control Panel");

            ui.separator();
            ui.horizontal_top(|ui| {
                if dev_started {
                    if ui.button("Stop virtual wheel").clicked() {
                        dev_started = false;
                    }
                } else {
                    if ui.button("Start virtual wheel").clicked() {
                        dev_started = true;
                    }
                }
            });

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

            ui.separator();
            dirty_wheel |= ui
                .add(
                    egui::Slider::new(&mut wheel.angle, -config.range..=config.range)
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
                });
            dirty_config |= config.source != old_source;

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
            dirty_config |= config.device != old_device;
        });

        egui::TopBottomPanel::bottom("steer_bar")
            .exact_height(32.0)
            .show(ctx, |ui| {
                let ui_rect = ui.min_rect();

                let centre = ui_rect.center().x;
                let bound = ui_rect.width() * 0.5;
                let mut min = 0.0;
                let mut max = (wheel.angle / config.range) * bound;
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
                        wheel.angle = remap(pos.x, left, right, -config.range, config.range);

                        dirty_wheel = true;
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // draw the (somewhat primitive) steering wheel

            let rect = ui.clip_rect();
            let origin = rect.center();
            let size = rect.size().x.min(rect.size().y) * 0.4;
            let colour = Color32::LIGHT_GRAY;
            let stroke = Stroke::new(size * 0.1, colour);
            let painter = ui.painter_at(rect);

            let sin = wheel.angle.to_radians().sin();
            let cos = wheel.angle.to_radians().cos();
            let right = Vec2::new(size * cos, size * sin);
            let down = Vec2::new(-size * sin, size * cos);

            painter.circle_stroke(origin, size, stroke);
            painter.circle_filled(origin, size * config.horn_radius, colour);
            painter.line_segment([origin + right, origin - right], stroke);
            painter.line_segment([origin, origin + down], stroke);
        });

        if dirty_config {
            state.lock().unwrap().config = config.clone();
        }

        if dirty_wheel {
            state.lock().unwrap().wheel = wheel.clone();
        }
    })
}

fn remap(t: f32, a1: f32, a2: f32, b1: f32, b2: f32) -> f32 {
    b1 + (t - a1) * (b2 - b1) / (a2 - a1)
}
