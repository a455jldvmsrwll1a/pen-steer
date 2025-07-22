use std::{fmt::Display, net::SocketAddr};

#[derive(Debug, Clone)]
pub struct Config {
    /// How many updates per second.
    pub update_frequency: u32,
    /// Angular range (in degrees) that the steering wheel may have each side.
    pub range: f32,
    /// Maximum threshold in which bringing the pen down triggers the horn.
    pub horn_radius: f32,
    /// Minimum units of pressure required for the pen to be considered touching.
    pub pressure_threshold: u32,
    /// Smallest radius in which angular velocity will be computed.
    pub base_radius: f32,

    /// Rotational inertia (in kg*m^2) of the simulated steering wheel.
    pub inertia: f32,

    /// Socket address to listen for data from, if using a `Net` source.
    pub net_sock_addr: String,

    /// Absolute axis resolution for the virtual device to present.
    pub output_resolution: u32,
    /// Virtual device name.
    pub device_name: String,
    /// Virtual device vendor.
    pub device_vendor: u32,
    /// Virtual device product.
    pub device_product: u32,
    /// Virtual device version.
    pub device_version: u32,

    pub source: Source,
    pub device: Device,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    None,
    Net,
    Wintab,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device {
    None,
    UInput,
    VigemBus,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            update_frequency: 120,
            range: 1800.0,
            horn_radius: 0.3,
            pressure_threshold: 10,
            base_radius: 0.6,
            inertia: 1.0, /* idk */
            net_sock_addr: "localhost:16027".into(),
            output_resolution: 32768,
            device_name: "G29 Driving Force Racing Wheel [PS3]".into(),
            device_vendor: 0x46D,
            device_product: 0xC24F,
            device_version: 0x3,
            #[cfg(target_os = "linux")]
            source: Source::Net,
            #[cfg(target_os = "windows")]
            source: Source::Wintab,
            #[cfg(target_os = "linux")]
            device: Device::UInput,
            #[cfg(target_os = "windows")]
            device: Device::VigemBus,
        }
    }
}

impl Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Source::None => "Disabled",
            Source::Net => "Network (over UDP)",
            Source::Wintab => "Wacom Wintab (Windows)",
        })
    }
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Device::None => "Null",
            Device::UInput => "Linux uinput",
            Device::VigemBus => "ViGEm Bus",
        })
    }
}