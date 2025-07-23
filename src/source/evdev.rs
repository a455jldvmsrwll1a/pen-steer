use anyhow::Result;

use crate::pen::Pen;

#[derive(Debug)]
pub struct EvdevSource {
    
}

impl EvdevSource {
    pub fn new(preferred_device_name: Option<&str>) -> Result<Self> {

        Ok(Self {
            
        })
    }

    pub fn try_read(&mut self) -> Option<Pen> {
        None
    }
}
