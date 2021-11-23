use petgraph::graph::EdgeIndex;

use crate::network::{FctRecord, SimNetwork};

/// An interface for link simulators.
// TODO: Add error reporting
pub trait LinkSim {
    /// Given a network and an edge (which will be a [`crate::network::types::TracedChannel`]),
    /// simulate the edge and return a collection of FCT records
    fn simulate(&self, network: &SimNetwork, edge: EdgeIndex) -> Vec<FctRecord>;
}
