use crate::network::DelayNetwork;
use crate::Spec;

/// The core `Parsimon` routine. This transforms a specification into a network of delay
/// distributions.
///
/// This function returns an error if the provided mappings in the specification are invalid.
pub fn run(spec: Spec) -> Result<DelayNetwork, Error> {
    // Validate mappings
    // Build a SimNetwork
    let simnet = spec.network.with_flows(todo!());
    // Use SimNetwork to run simulations
    // Aggregate simulation results into a DelayNet
    todo!()
}

#[derive(Debug)]
pub enum Error {}
