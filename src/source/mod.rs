pub mod net;

use crate::{pen::Pen, source::net::NetSource};

#[derive(Debug, Default)]
pub enum Source {
    /// Dummy source, does nothing.
    #[default]
    Dummy,
    /// Receive input events from external software via network.
    Net(NetSource),
}

impl Source {
    pub fn get(&mut self) -> Option<Pen> {
        match self {
            Source::Dummy => None,
            Source::Net(net_source) => net_source.try_read(),
        }
    }
}
