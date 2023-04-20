//! This module defines the [`LinkSim`] trait that every link simulator must implement.

use petgraph::graph::EdgeIndex;

use crate::network::{FctRecord, SimNetwork};

/// The return type of a link simulation.
pub type LinkSimResult = Result<Vec<FctRecord>, LinkSimError>;

/// An interface for link simulators.
pub trait LinkSim {
    /// Given a network and an edge (which will be a [`crate::network::types::FlowChannel`]),
    /// simulate the edge and return a collection of FCT records.
    fn simulate(&self, network: &SimNetwork, edge: EdgeIndex) -> LinkSimResult;
}

impl<T: LinkSim> LinkSim for &T {
    fn simulate(&self, network: &SimNetwork, edge: EdgeIndex) -> LinkSimResult {
        (*self).simulate(network, edge)
    }
}

/// A full specification for a link-level simulation.
#[derive(Debug)]
pub struct LinkSimSpec {}

/// A descriptor for a link-level simulation.
#[derive(Debug)]
pub(crate) struct LinkSimDesc {}

/// Link simulation error.
#[derive(Debug, thiserror::Error)]
pub enum LinkSimError {
    /// Tried to simulate a link that doesn't exist.
    #[error("Edge {} does not exist", .0.index())]
    UnknownEdge(EdgeIndex),

    /// IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Arbitrary catch-all.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
