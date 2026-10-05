/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use glam::Vec2;

#[derive(Debug, Default, Clone)]
pub struct Profile {
    tick_mode: TickMode,
    active_control: ControlType,
    active_output: OutputBackend,
    active_input: InputBackend,

    mapping: Mapping,

    dummy_control: DummyControl,
    steering_wheel_control: SteeringWheelControl,
    one_tablet_drive_control: OneTabletDriveControl,

    net_input: NetInput,
    linux_evdev_input: LinuxEvdevInput,
    windows_wintab_input: WindowsWintabInput,

    linux_uinput_output: LinuxUinputOutput,
    linux_usb_peripheral_output: LinuxUsbPeripheralOutput,
    windows_vigembus_output: WindowsVigemBusOutput,
    usb_over_ip_server_output: UsbOverIpServerOutput,
}

//
// MAPPING
//

#[derive(Debug, Clone)]
pub struct Mapping {
    pub translation: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

//
// CONTROLLER
//

#[derive(Debug, Default, Clone)]
pub enum TickMode {
    #[default]
    Automatic,
    Lockstep,
    Fixed {
        frequency: f32,
    },
}

#[derive(Debug, Default, Clone)]
pub enum ControlType {
    Dummy,
    #[default]
    SteeringWheel,
    OneTabletDrive,
}

#[derive(Debug, Default, Clone)]
pub struct DummyControl {
    steer_value: f32,
    accelerator_value: f32,
    brake_value: f32,
    horn: bool,
}

#[derive(Debug, Clone)]
pub struct SteeringWheelControl {
    angle_range: f32,
    natural_rotation_radius_min: f32,
    horn_radius: f32,
    pressure_min: f32,
    pressure_max: f32,
    ignore_exterior_input: bool,
    sticky_horn: bool,
    fast_mode_switch: bool,
    physics_enable: bool,
    inertia: f32,
    friction_coeff: f32,
    spring_stiffness: f32,
    allow_slippage: bool,
    auto_grip_force: bool,
    vary_grip_arm_length: bool,
    direct_grip_threshold: f32,
    full_grip_force: f32,
    enable_ffb: bool,
    full_ffb_negate: bool,
    full_ffb_force: f32,
}

#[derive(Debug, Default, Clone)]
pub struct OneTabletDriveControl {}

//
// INPUT
//

#[derive(Debug, Default, Clone)]
pub enum InputBackend {
    #[default]
    Null,
    Net,
    LinuxEvdev,
    WindowsWintab,
}

#[derive(Debug, Default, Clone)]
pub struct NetInput {
    listen_addr: String,
}

#[derive(Debug, Default, Clone)]
pub struct LinuxEvdevInput {}

#[derive(Debug, Default, Clone)]
pub struct WindowsWintabInput {}

//
// OUTPUT
//

#[derive(Debug, Default, Clone)]
pub enum OutputBackend {
    #[default]
    Null,
    LinuxUinput,
    LinuxUsbPeripheral,
    WindowsVigemBus,
    UsbOverIpServer,
}

#[derive(Debug, Default, Clone)]
pub struct LinuxUinputOutput {}

#[derive(Debug, Default, Clone)]
pub struct LinuxUsbPeripheralOutput {}

#[derive(Debug, Default, Clone)]
pub struct WindowsVigemBusOutput {}

#[derive(Debug, Default, Clone)]
pub struct UsbOverIpServerOutput {}

impl Default for Mapping {
    fn default() -> Self {
        Self {
            translation: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
        }
    }
}

impl Default for SteeringWheelControl {
    fn default() -> Self {
        Self {
            angle_range: 1080.0,
            natural_rotation_radius_min: 60.0,
            horn_radius: 30.0,
            pressure_min: 5.0,
            pressure_max: 40.0,
            ignore_exterior_input: false,
            sticky_horn: false,
            fast_mode_switch: false,
            physics_enable: true,
            inertia: 20.0,
            friction_coeff: 100.0,
            spring_stiffness: 0.0,
            allow_slippage: true,
            auto_grip_force: true,
            vary_grip_arm_length: false,
            direct_grip_threshold: 80.0,
            full_grip_force: 2000.0,
            enable_ffb: true,
            full_ffb_negate: false,
            full_ffb_force: 900.0,
        }
    }
}
