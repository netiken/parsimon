mod routing;
pub(crate) mod topology;
pub(crate) mod types;

use crate::{client::Flow, Link, Node, TopologyError};

use self::{
    routing::Routes,
    topology::Topology,
    types::{Channel, TracedChannel},
};

#[derive(Debug)]
pub struct Network {
    topology: Topology<Channel>,
    routes: Routes,
}

impl Network {
    pub fn new(nodes: &[Node], links: &[Link]) -> Result<Self, TopologyError> {
        let topology = Topology::new(nodes, links)?;
        let routes = Routes::new(&topology);
        Ok(Self { topology, routes })
    }
}

#[derive(Debug)]
pub(crate) struct SimNetwork {
    topology: Topology<TracedChannel>,
    routes: Routes,
}

impl SimNetwork {
    /// Create a `SimNetwork`.
    ///
    /// PRECONDITION: For each flow in `flows`, `flow.src` and `flow.dst` must be valid hosts in
    /// `network`.
    pub(crate) fn new(network: &Network, flows: Vec<Flow>) -> Self {
        Self {
            topology: todo!(),
            routes: network.routes.clone(),
        }
    }
}

#[derive(Debug)]
pub struct DelayNetwork;
