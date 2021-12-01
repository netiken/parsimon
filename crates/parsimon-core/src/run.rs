use crate::linksim::LinkSim;
use crate::network::{DelayNetwork, SimNetworkError};
use crate::spec::{Spec, SpecError};

/// The core `Parsimon` routine. This transforms a specification into a network of delay
/// distributions.
///
/// This function returns an error if the provided mappings in the specification are invalid.
pub fn run<S: LinkSim>(spec: Spec<S>) -> Result<DelayNetwork, Error> {
    let spec = spec.validate()?;
    let flows = spec.collect_flows();
    let sims = spec.network.into_simulations(flows);
    let delays = sims.into_delays(spec.linksim)?;
    Ok(delays)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid specification")]
    InvalidSpec(#[from] SpecError),

    #[error("SimNetwork error")]
    SimNetwork(#[from] SimNetworkError),
}
