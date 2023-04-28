//! This module defines the [`run`] routine, which is `Parsimon`'s main entry point.

use crate::cluster::ClusteringAlgo;
use crate::linksim::LinkSim;
use crate::network::{DelayNetwork, SimNetworkError};
use crate::opts::SimOpts;
use crate::spec::{Spec, SpecError};

/// The core `Parsimon` routine. This transforms a specification into a network of delay
/// distributions, using a provided [link simulation options](SimOpts) and [clustering algorithm](ClusteringAlgo).
pub fn run<S, C>(spec: Spec, opts: SimOpts<S>, clusterer: C) -> Result<DelayNetwork, Error>
where
    S: LinkSim + Sync,
    C: ClusteringAlgo,
{
    let spec = spec.validate()?;
    let flows = spec.collect_flows();
    let mut sims = spec.network.into_simulations(flows);
    sims.cluster(clusterer);
    let delays = sims.into_delays(opts)?;
    Ok(delays)
}

/// The error type for the core [run] routine.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid specification.
    #[error("Invalid specification")]
    InvalidSpec(#[from] SpecError),

    /// Error running the simulations.
    #[error("SimNetwork error")]
    SimNetwork(#[from] SimNetworkError),
}
