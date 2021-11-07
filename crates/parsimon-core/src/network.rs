mod routing;
pub(crate) mod topology;
pub(crate) mod types;

use petgraph::graph::EdgeIndex;
use rand::{seq::SliceRandom, Rng};

use crate::{client::Flow, utils, Link, Node, TopologyError};

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

    /// Create a `SimNetwork`.
    ///
    /// PRECONDITION: For each flow in `flows`, `flow.src` and `flow.dst` must be valid hosts in
    /// `network`.
    pub(crate) fn sim_network(&self, flows: Vec<Flow>) -> SimNetwork {
        let mut topology = Topology::<TracedChannel>::new_empty(&self.topology);
        for Flow { id, src, dst, .. } in flows {
            let hash = utils::calculate_hash(&id);
            let path = self.edges_indices_between(src, dst, |choices| {
                let idx = hash as usize % choices.len();
                Some(&choices[idx])
            });
            for eidx in path {
                topology.graph[eidx].push_flow(id);
            }
        }
        SimNetwork {
            topology,
            routes: self.routes.clone(),
        }
    }

    pub(crate) fn edges_indices_between(
        &self,
        src: NodeId,
        dst: NodeId,
        choose: impl Fn(&[NodeId]) -> Option<&NodeId>,
    ) -> impl Iterator<Item = EdgeIndex> {
        let mut acc = Vec::new();
        let mut cur = src;
        while cur != dst {
            let next_hop_choices = self.routes.next_hops_unchecked(cur, dst);
            match choose(next_hop_choices) {
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

impl SimNetwork {}

#[derive(Debug)]
pub struct DelayNetwork;
