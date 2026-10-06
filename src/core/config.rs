/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use glam::Vec2;

#[derive(Debug, Default, Clone)]
pub struct Profile {
    pub tick_mode: TickMode,
    pub active_control: ControlType,
    pub active_output: OutputBackend,
    pub active_input: InputBackend,

    pub mapping: Mapping,

    pub dummy_control: DummyControl,
    pub steering_wheel_control: SteeringWheelControl,
    pub one_tablet_drive_control: OneTabletDriveControl,

    pub net_input: NetInput,
    pub linux_evdev_input: LinuxEvdevInput,
    pub windows_wintab_input: WindowsWintabInput,

    pub linux_uinput_output: LinuxUinputOutput,
    pub linux_usb_peripheral_output: LinuxUsbPeripheralOutput,
    pub windows_vigembus_output: WindowsVigemBusOutput,
    pub usb_over_ip_server_output: UsbOverIpServerOutput,
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
    pub steer_value: f32,
    pub accelerator_value: f32,
    pub brake_value: f32,
    pub horn: bool,
}

#[derive(Debug, Clone)]
pub struct SteeringWheelControl {
    pub angle_range: f32,
    pub natural_rotation_radius_min: f32,
    pub horn_radius: f32,
    pub pressure_min: f32,
    pub pressure_max: f32,
    pub ignore_exterior_input: bool,
    pub sticky_horn: bool,
    pub fast_mode_switch: bool,
    pub physics_enable: bool,
    pub inertia: f32,
    pub friction_coeff: f32,
    pub spring_stiffness: f32,
    pub allow_slippage: bool,
    pub auto_grip_force: bool,
    pub vary_grip_arm_length: bool,
    pub direct_grip_threshold: f32,
    pub full_grip_force: f32,
    pub enable_ffb: bool,
    pub full_ffb_negate: bool,
    pub full_ffb_force: f32,
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

#[derive(Debug, Clone)]
pub struct NetInput {
    pub listen_addr: String,
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

impl Default for NetInput {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:16027".into(),
        }
    }
}
