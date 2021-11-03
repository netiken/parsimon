mod routing;
pub(crate) mod topology;
pub(crate) mod types;

use petgraph::graph::EdgeIndex;
use rand::{seq::SliceRandom, Rng};

use crate::{client::Flow, Link, Node, TopologyError};

use self::{
    routing::Routes,
    topology::Topology,
    types::{Channel, NodeId, TracedChannel},
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

    // TODO: turn this into something else
    // pub(crate) fn into_

    pub(crate) fn edges_indices_between(
        &self,
        src: NodeId,
        dst: NodeId,
        mut rng: impl Rng,
    ) -> impl Iterator<Item = EdgeIndex> {
        let mut acc = Vec::new();
        let mut cur = src;
        while cur != dst {
            let next_hop_choices = self.routes.next_hops_unchecked(cur, dst);
            match next_hop_choices.choose(&mut rng) {
                Some(&next_hop) => {
                    // These indices are all guaranteed to exist because we have a valid topology
                    let i = *self.topology.idx_of(&cur).unwrap();
                    let j = *self.topology.idx_of(&next_hop).unwrap();
                    let e = self.topology.find_edge(i, j).unwrap();
                    acc.push(e);
                    cur = next_hop;
                }
                None => return Vec::new().into_iter(),
            }
        }
        acc.into_iter()
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
