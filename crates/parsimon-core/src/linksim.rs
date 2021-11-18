use petgraph::graph::EdgeIndex;

use crate::{edist::EDistBuckets, network::SimNetwork};

/// An interface for link simulators.
pub trait LinkSim {
    /// Given a network and an edge (which will be a [`crate::TracedChannel`]), simulate the edge
    /// and return a collection of delay distributions bucketed by flow size.
    fn simulate(&self, network: &SimNetwork, edge: EdgeIndex) -> EDistBuckets;
}
