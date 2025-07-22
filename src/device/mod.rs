#[derive(Debug, Default)]
pub enum Device {
    /// Dummy device, does nothing.
    #[default]
    Dummy,
    /// Presents a virtual device using Linux's uinput.
    UInput,
}
