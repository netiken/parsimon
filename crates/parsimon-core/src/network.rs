mod routing;
mod topology;
pub(crate) mod types;

use self::{routing::Routes, topology::Topology};

#[derive(Debug)]
pub struct Network {
    topology: Topology,
    routes: Routes,
}

impl Network {
    pub fn new() -> Self {
        todo!()
    }
}

#[derive(Debug)]
pub struct DelayNet;
