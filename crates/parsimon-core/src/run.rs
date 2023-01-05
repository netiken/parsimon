use crate::cluster::ClusteringAlgo;
use crate::linksim::LinkSim;
use crate::network::{DelayNetwork, SimNetworkError};
use crate::spec::{Spec, SpecError};

/// The core `Parsimon` routine. This transforms a specification into a network of delay
/// distributions.
///
/// This function returns an error if the provided mappings in the specification are invalid.
pub fn run<S, C>(spec: Spec, linksim: S, clusterer: C) -> Result<DelayNetwork, Error>
where
    S: LinkSim + Sync,
    C: ClusteringAlgo,
{
    let spec = spec.validate()?;
    let flows = spec.collect_flows();
    let mut sims = spec.network.into_simulations(flows);
    sims.cluster(clusterer);
    let delays = sims.into_delays(linksim)?;
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
