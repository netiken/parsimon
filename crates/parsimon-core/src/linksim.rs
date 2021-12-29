use petgraph::graph::EdgeIndex;

use crate::network::{FctRecord, SimNetwork};

pub type LinkSimResult = Result<Vec<FctRecord>, LinkSimError>;

/// An interface for link simulators.
pub trait LinkSim {
    /// Given a network and an edge (which will be a [`crate::network::types::TracedChannel`]),
    /// simulate the edge and return a collection of FCT records.
    fn simulate(&self, network: &SimNetwork, edge: EdgeIndex) -> LinkSimResult;
}

impl<T: LinkSim> LinkSim for &T {
    fn simulate(&self, network: &SimNetwork, edge: EdgeIndex) -> LinkSimResult {
        (*self).simulate(network, edge)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LinkSimError {
    #[error("Edge {} does not exist", .0.index())]
    UnknownEdge(EdgeIndex),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
