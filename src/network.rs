// A topology that we know is valid.
// Includes topology and routes
// - This is probably enough to replicate ECMP decisions
#[derive(Debug)]
pub struct Network {
    topology: (), // maybe this should just be a graph
    routes: (),   // copy this from cloudburst
}

impl Network {}
