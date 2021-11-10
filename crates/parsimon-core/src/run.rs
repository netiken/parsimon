use crate::network::DelayNetwork;
use crate::spec::{Spec, SpecError};

/// The core `Parsimon` routine. This transforms a specification into a network of delay
/// distributions.
///
/// This function returns an error if the provided mappings in the specification are invalid.
pub fn run(spec: Spec) -> Result<DelayNetwork, Error> {
    let spec = spec.validate()?;
    let flows = spec.collect_flows();
    let _network = spec.network.with_flows(flows);
    // Use SimNetwork to run simulations
    // Aggregate simulation results into a DelayNet
    todo!()
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    InvalidSpec(#[from] SpecError),
}
