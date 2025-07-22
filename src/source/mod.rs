pub mod net;

use crate::source::net::NetSource;

#[derive(Debug, Default)]
pub enum Source {
    /// Dummy source, does nothing.
    #[default]
    Dummy,
    /// Receive input events from external software via network.
    Net(NetSource),
}
