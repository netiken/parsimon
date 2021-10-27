use crate::network::DelayNet;
use crate::Spec;

/// The core `Parsimon` routine. This transforms a specification into a network of delay
/// distributions.
///
/// This function returns an error if the provided mappings in the specification are invalid.
pub fn run(_spec: Spec) -> Result<DelayNet, Error> {
    // Validate mappings
    // Build a FlowNet
    // Use FlowNet to run simulations
    // Aggregate simulation results into a DelayNet
    todo!()
}

#[derive(Debug)]
pub enum Error {}
