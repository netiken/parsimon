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
                    let e = self.topology.graph.find_edge(i, j).unwrap();
                    acc.push(e);
                    cur = next_hop;
                }
                // There is no choice of next hop, and therefore no path
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

#[cfg(test)]
mod tests {
    use anyhow::Context;

    use crate::{
        client::{ClientId, FlowId, UniqFlowId},
        testing,
    };

    use super::*;

    // This test creates an eight-node topology and sends some flows with the
    // same source and destination across racks. All flows will traverse
    // exactly one ECMP group in the upwards direction. While we don't know
    // exactly how many flows should traverse each link in the group, we can
    // check that the final counts are close to equal. If a different hashing
    // algorithm is used, the exact counts will change, and this test will need
    // to be updated to use a different snapshot.
    #[test]
    fn ecmp_replication_works() -> anyhow::Result<()> {
        let (nodes, links) = testing::eight_node_config();
        let network = Network::new(&nodes, &links).context("failed to create topology")?;
        let flows = (0..100)
            .map(|i| Flow {
                id: UniqFlowId::new(ClientId::new(0), FlowId::new(i)),
                src: NodeId::new(0),
                dst: NodeId::new(3),
                size: 0,
                start: 0,
            })
            .collect::<Vec<_>>();
        let network = network.sim_network(flows);

        // The ECMP group contains edges (4, 6) and (4, 7)
        let e1 = network
            .topology
            .find_edge(NodeId::new(4), NodeId::new(6))
            .unwrap();
        let e2 = network
            .topology
            .find_edge(NodeId::new(4), NodeId::new(7))
            .unwrap();

        // Flow counts for the links should be close to each other
        insta::assert_yaml_snapshot!((
            network.topology.graph[e1].flows.len(),
            network.topology.graph[e2].flows.len(),
        ), @r###"
        ---
        - 42
        - 58
        "###);

        Ok(())
    }
}
