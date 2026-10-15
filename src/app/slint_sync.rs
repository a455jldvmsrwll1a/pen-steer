/*
 * SPDX-FileCopyrightText: 2026 a455jldvmsrwll1a
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use pen_steer::core::config::{
    self, DummyControl, Mapping, NetInput, OneTabletDriveControl, Profile, SteeringWheelControl,
};
use slint::ToSharedString;

use crate::{ControlConfig, GlobalUiState, InputConfig};

// if only you could export native structs to slint :(
// this is frankly ugly as hell

pub fn update_profile_from_control(profile: &mut Profile, control: &ControlConfig) {
    profile.tick_mode = match control.freq_mode {
        crate::UpdateFrequencyMode::Automatic => config::TickMode::Automatic,
        crate::UpdateFrequencyMode::Lockstep => config::TickMode::Lockstep,
        crate::UpdateFrequencyMode::Fixed => config::TickMode::Fixed {
            frequency: control.fixed_freq,
        },
    };

    profile.active_control = match control.active {
        crate::ControlType::Dummy => config::ControlType::Dummy,
        crate::ControlType::SteeringWheel => config::ControlType::SteeringWheel,
        crate::ControlType::OneTabletDrive => config::ControlType::OneTabletDrive,
    };

    // profile.active_output = match control.output_backend {
    //     crate::OutputBackend::Null => pen_steer::core::config::OutputBackend::Null,
    //     crate::OutputBackend::LinuxUinput => pen_steer::core::config::OutputBackend::LinuxUinput,
    //     crate::OutputBackend::WindowsVigem => {
    //         pen_steer::core::config::OutputBackend::WindowsVigemBus
    //     }
    //     crate::OutputBackend::UsbOverIp => pen_steer::core::config::OutputBackend::UsbOverIpServer,
    // };

    profile.mapping = Mapping::default();

    profile.dummy_control = DummyControl::default();
    profile.steering_wheel_control = SteeringWheelControl {
        angle_range: control.steering_wheel.angle_range,
        natural_rotation_radius_min: control.steering_wheel.natural_rotation_radius_min,
        horn_radius: control.steering_wheel.horn_radius,
        pressure_min: control.steering_wheel.pressure_min,
        pressure_max: control.steering_wheel.pressure_max,
        ignore_exterior_input: control.steering_wheel.ignore_exterior_input,
        sticky_horn: control.steering_wheel.sticky_horn,
        fast_mode_switch: control.steering_wheel.fast_mode_switch,
        physics_enable: control.steering_wheel.physics_enable,
        inertia: control.steering_wheel.inertia,
        friction_coeff: control.steering_wheel.friction_coeff,
        spring_stiffness: control.steering_wheel.spring_stiffness,
        allow_slippage: control.steering_wheel.allow_slippage,
        auto_grip_force: control.steering_wheel.auto_grip_force,
        vary_grip_arm_length: control.steering_wheel.vary_grip_arm_length,
        direct_grip_threshold: control.steering_wheel.direct_grip_threshold,
        full_grip_force: control.steering_wheel.full_grip_force,
        enable_ffb: control.steering_wheel.enable_ffb,
        full_ffb_negate: control.steering_wheel.full_ffb_negate,
        full_ffb_force: control.steering_wheel.full_ffb_force,
    };
    profile.one_tablet_drive_control = OneTabletDriveControl::default();
}

pub fn update_profile_from_input(profile: &mut Profile, input: &InputConfig) {
    profile.active_input = match input.active {
        crate::InputBackend::Null => config::InputBackend::Null,
        crate::InputBackend::Net => config::InputBackend::Net,
        crate::InputBackend::LinuxEvdev => config::InputBackend::LinuxEvdev,
        crate::InputBackend::WindowsWintab => config::InputBackend::WindowsWintab,
    };

    profile.net_input = NetInput {
        listen_addr: input.net_config.listen_address.clone().into(),
    }
}

pub fn update_global_from_profile(global: &mut GlobalUiState<'_>, profile: &Profile) {
    let (freq_mode, fixed_freq) = match profile.tick_mode {
        config::TickMode::Automatic => (crate::UpdateFrequencyMode::Automatic, 60.0),
        config::TickMode::Lockstep => (crate::UpdateFrequencyMode::Lockstep, 60.0),
        config::TickMode::Fixed { frequency } => (crate::UpdateFrequencyMode::Fixed, frequency),
    };

    global.set_control_config(ControlConfig {
        active: match profile.active_control {
            config::ControlType::Dummy => crate::ControlType::Dummy,
            config::ControlType::SteeringWheel => crate::ControlType::SteeringWheel,
            config::ControlType::OneTabletDrive => crate::ControlType::OneTabletDrive,
        },
        dummy: crate::DummyConfig {},
        fixed_freq,
        freq_mode,
        input_backend: crate::InputBackend::Null,
        one_tablet_drive: crate::OneTabletDriveConfig {},
        output_backend: crate::OutputBackend::Null,
        steering_wheel: crate::SteeringWheelConfig {
            allow_slippage: profile.steering_wheel_control.allow_slippage,
            angle_range: profile.steering_wheel_control.angle_range,
            auto_grip_force: profile.steering_wheel_control.auto_grip_force,
            direct_grip_threshold: profile.steering_wheel_control.direct_grip_threshold,
            enable_ffb: profile.steering_wheel_control.enable_ffb,
            fast_mode_switch: profile.steering_wheel_control.fast_mode_switch,
            friction_coeff: profile.steering_wheel_control.friction_coeff,
            full_ffb_force: profile.steering_wheel_control.full_ffb_force,
            full_ffb_negate: profile.steering_wheel_control.full_ffb_negate,
            full_grip_force: profile.steering_wheel_control.full_grip_force,
            horn_radius: profile.steering_wheel_control.horn_radius,
            ignore_exterior_input: profile.steering_wheel_control.ignore_exterior_input,
            inertia: profile.steering_wheel_control.inertia,
            natural_rotation_radius_min: profile.steering_wheel_control.natural_rotation_radius_min,
            physics_enable: profile.steering_wheel_control.physics_enable,
            pressure_max: profile.steering_wheel_control.pressure_max,
            pressure_min: profile.steering_wheel_control.pressure_min,
            spring_stiffness: profile.steering_wheel_control.spring_stiffness,
            sticky_horn: profile.steering_wheel_control.sticky_horn,
            vary_grip_arm_length: profile.steering_wheel_control.vary_grip_arm_length,
        },
    });

    global.set_input_config(InputConfig {
        active: match profile.active_input {
            config::InputBackend::Null => crate::InputBackend::Null,
            config::InputBackend::Net => crate::InputBackend::Net,
            config::InputBackend::LinuxEvdev => crate::InputBackend::LinuxEvdev,
            config::InputBackend::WindowsWintab => crate::InputBackend::WindowsWintab,
        },
        net_config: crate::NetInputBackendConfig {
            listen_address: profile.net_input.listen_addr.to_shared_string(),
        },
    });
}
