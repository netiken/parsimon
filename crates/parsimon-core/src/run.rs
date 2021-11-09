use crate::network::DelayNetwork;
use crate::spec::{Spec, SpecError};

/// The core `Parsimon` routine. This transforms a specification into a network of delay
/// distributions.
///
/// This function returns an error if the provided mappings in the specification are invalid.
pub fn run(spec: Spec) -> Result<DelayNetwork, Error> {
    // Validate mappings
    let spec = spec.validate()?;
    // Build a SimNetwork
    // let simnet = spec.network.with_flows(todo!());
    // Use SimNetwork to run simulations
    // Aggregate simulation results into a DelayNet
    todo!()
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    InvalidSpec(#[from] SpecError),
}
