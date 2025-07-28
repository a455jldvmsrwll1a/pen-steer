use anyhow::Result;
use log::info;

use crate::device::Device;

pub struct VigemDevice {

}

impl VigemDevice {
    pub fn new() -> Result<Self> {
        info!("Vigem device initialised!");

        Ok(Self {

        })
    }
}

impl Device for VigemDevice {
    fn get_feedback(&self) -> Option<f32> {
        None
    }

    fn set_wheel(&mut self, angle: f32) {
        info!("Vigem set_wheel({angle})");
    }

    fn set_horn(&mut self, honking: bool) {
        info!("Vigem set_horn{honking})");
    }

    fn apply(&mut self) -> Result<()> {
        Ok(())
    }

    fn handle_events(&mut self) {
        
    }
}