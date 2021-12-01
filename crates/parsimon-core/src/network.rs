mod routing;
pub(crate) mod topology;
pub mod types;

pub use petgraph::graph::EdgeIndex;
pub use topology::TopologyError;
pub use types::*;

use crate::{
    edist::EDistError,
    linksim::{LinkSim, LinkSimError},
    utils,
};

use self::{
    routing::Routes,
    topology::Topology,
    types::{Channel, EDistChannel, Link, Node, TracedChannel},
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
    /// PRECONDITIONS: For each flow in `flows`, `flow.src` and `flow.dst` must be valid hosts in
    /// `network`.
    pub fn into_simulations(self, mut flows: Vec<Flow>) -> SimNetwork {
        flows.sort_by_key(|f| f.start);
        let mut topology = Topology::new_traced(&self.topology);
        for &Flow { id, src, dst, .. } in &flows {
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
            flows,
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

    delegate::delegate! {
        to self.topology.graph {
            #[call(node_weights)]
            pub fn nodes(&self) -> impl Iterator<Item = &Node>;
        }

        to self.topology.links {
            #[call(iter)]
            pub fn links(&self) -> impl Iterator<Item = &Link>;
        }
    }
}

#[derive(Debug)]
pub struct SimNetwork {
    topology: Topology<TracedChannel>,
    routes: Routes,
    flows: Vec<Flow>,
}

impl SimNetwork {
    pub fn into_delays<S: LinkSim>(self, sim: S) -> Result<DelayNetwork, SimNetworkError> {
        let mut topology = Topology::new_edist(&self.topology);
        for eidx in self.topology.graph.edge_indices() {
            // TODO: This should happen in parallel (either with a pool or with rayon)
            let data = sim.simulate(&self, eidx)?;
            topology.graph[eidx]
                .dists
                .fill(&data, |rec| rec.size, |rec| rec.pktnorm_delay())?;
        }
        Ok(DelayNetwork {
            topology,
            routes: self.routes.clone(),
        })
    }

    delegate::delegate! {
        to self.topology.graph {
            #[call(node_weights)]
            pub fn nodes(&self) -> impl Iterator<Item = &Node>;

            #[call(edge_weight)]
            pub fn edge(&self, idx: EdgeIndex) -> Option<&TracedChannel>;
        }

        to self.topology.links {
            #[call(iter)]
            pub fn links(&self) -> impl Iterator<Item = &Link>;
        }

        to self.flows {
            #[call(iter)]
            pub fn flows(&self) -> impl Iterator<Item = &Flow>;
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SimNetworkError {
    #[error("Failed to simulate link")]
    LinkSim(#[from] LinkSimError),

    #[error("Failed to construct empirical distribution")]
    EDist(#[from] EDistError),
}

#[derive(Debug)]
pub struct DelayNetwork {
    topology: Topology<EDistChannel>,
    routes: Routes,
}

impl DelayNetwork {
    delegate::delegate! {
        to self.topology.graph {
            #[call(node_weights)]
            pub fn nodes(&self) -> impl Iterator<Item = &Node>;
        }

        to self.topology.links {
            #[call(iter)]
            pub fn links(&self) -> impl Iterator<Item = &Link>;
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Context;

    use crate::client::ClientId;
    use crate::network::{FlowId, UniqFlowId};
    use crate::testing;
    use crate::units::{Bytes, Nanosecs};

    use super::*;

    fn find_edge<C>(topo: &Topology<C>, src: NodeId, dst: NodeId) -> Option<EdgeIndex> {
        topo.idx_of(&src)
            .and_then(|&a| topo.idx_of(&dst).map(|&b| (a, b)))
            .and_then(|(a, b)| topo.graph.find_edge(a, b))
    }

    // This test creates an eight-node topology and sends some flows with the
    // same source and destination across racks. All flows will traverse
    // exactly one ECMP group in the upwards direction. While we don't know
    // exactly how many flows should traverse each link in the group, we can
    // check that the final counts are close to equal. If a different hashing
    // algorithm is used, the exact counts will change, and this test will need
    // to be updated to use the new snapshot.
    #[test]
    fn ecmp_replication_works() -> anyhow::Result<()> {
        let (nodes, links) = testing::eight_node_config();
        let network = Network::new(&nodes, &links).context("failed to create topology")?;
        let flows = (0..100)
            .map(|i| Flow {
                id: UniqFlowId::new(ClientId::new(0), FlowId::new(i)),
                src: NodeId::new(0),
                dst: NodeId::new(3),
                size: Bytes::ZERO,
                start: Nanosecs::ZERO,
            })
            .collect::<Vec<_>>();
        let network = network.into_simulations(flows);

        // The ECMP group contains edges (4, 6) and (4, 7)
        let e1 = find_edge(&network.topology, NodeId::new(4), NodeId::new(6)).unwrap();
        let e2 = find_edge(&network.topology, NodeId::new(4), NodeId::new(7)).unwrap();

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
