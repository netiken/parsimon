mod routing;
pub(crate) mod topology;
pub mod types;

use std::collections::HashMap;

use rand::prelude::*;
use rayon::prelude::*;

pub use petgraph::graph::EdgeIndex;
pub use topology::TopologyError;
pub use types::*;

use crate::{
    cluster::{Cluster, ClusteringAlgo},
    edist::EDistError,
    linksim::{LinkSim, LinkSimError},
    units::{Bytes, Nanosecs},
    utils,
};

use self::{
    routing::Routes,
    topology::Topology,
    types::{Channel, EDistChannel, FlowChannel, Link, Node},
};

#[derive(Debug, Clone)]
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
    /// `network`, and there must be a path between them.
    pub fn into_simulations(self, mut flows: Vec<Flow>) -> SimNetwork {
        flows.sort_by_key(|f| f.start);
        let mut topology = Topology::new_traced(&self.topology);
        for &Flow { id, src, dst, .. } in &flows {
            let hash = utils::calculate_hash(&id);
            let path = self.edge_indices_between(src, dst, |choices| {
                assert!(!choices.is_empty(), "missing path from {} to {}", src, dst);
                let idx = hash as usize % choices.len();
                Some(&choices[idx])
            });
            for eidx in path {
                topology.graph[eidx].push_flow(id);
            }
        }
        // The default clustering uses a 1:1 mapping between edges and clusters.
        // CORRECTNESS: The code below assumes edge indices start at zero.
        let clusters = topology
            .graph
            .edge_indices()
            .map(|eidx| Cluster::new(eidx, [eidx].into_iter().collect()))
            .collect();
        SimNetwork {
            topology,
            routes: self.routes.clone(),
            clusters,
            flows: flows.into_iter().map(|f| (f.id, f)).collect(),
        }
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

impl TraversableNetwork<Channel> for Network {
    fn topology(&self) -> &Topology<Channel> {
        &self.topology
    }

    fn routes(&self) -> &Routes {
        &self.routes
    }
}

#[derive(Debug, Clone)]
pub struct SimNetwork {
    topology: Topology<FlowChannel>,
    routes: Routes,

    // Channel clustering
    clusters: Vec<Cluster>,
    // Each channel references these flows by ID
    flows: HashMap<FlowId, Flow>,
}

impl SimNetwork {
    pub fn cluster<C>(&mut self, algorithm: C)
    where
        C: ClusteringAlgo,
    {
        let clusters = algorithm.cluster(self);
        self.clusters = clusters;
    }

    pub fn into_delays<S>(self, sim: S) -> Result<DelayNetwork, SimNetworkError>
    where
        S: LinkSim + Sync,
    {
        let mut topology = Topology::new_edist(&self.topology);
        let eidx2data = self.simulate_clusters(sim)?;
        // Every channel gets filled with delay distributions. All channels in the same cluster get
        // filled using the cluster representative's data.
        for cluster in &self.clusters {
            let representative = cluster.representative();
            for &member in cluster.members() {
                // Fill channel with packet-normalized delay predictions
                let data = eidx2data.get(&representative).unwrap();
                topology.graph[member].dists.fill(
                    data,
                    |rec| rec.size,
                    |rec| rec.pktnorm_delay(),
                )?;
            }
        }
        Ok(DelayNetwork {
            topology,
            routes: self.routes.clone(),
        })
    }

    pub fn simulate_clusters<S>(
        &self,
        sim: S,
    ) -> Result<HashMap<EdgeIndex, Vec<FctRecord>>, SimNetworkError>
    where
        S: LinkSim + Sync,
    {
        let (s, r) = crossbeam_channel::unbounded();
        // Simulate all clusters in parallel.
        self.clusters.par_iter().try_for_each_with(s, |s, c| {
            let edge = c.representative();
            let data = sim.simulate(self, edge)?;
            s.send((edge, data)).unwrap(); // the channel should never become disconnected
            Result::<(), SimNetworkError>::Ok(())
        })?;
        Ok(r.iter().collect())
    }

    pub fn flows_on(&self, edge: EdgeIndex) -> Option<Vec<Flow>> {
        self.edge(edge).map(|chan| {
            chan.flow_ids()
                .map(|id| self.flows.get(&id).unwrap().to_owned().to_owned())
                .collect()
        })
    }

    /// Get a reference to the sim network's clusters.
    pub fn clusters(&self) -> &[Cluster] {
        self.clusters.as_ref()
    }

    /// Set the sim network's clusters.
    pub fn set_clusters(&mut self, clusters: Vec<Cluster>) {
        self.clusters = clusters;
    }

    delegate::delegate! {
        to self.topology.graph {
            #[call(node_weights)]
            pub fn nodes(&self) -> impl Iterator<Item = &Node>;

            #[call(edge_weight)]
            pub fn edge(&self, idx: EdgeIndex) -> Option<&FlowChannel>;

            #[call(edge_weights)]
            pub fn edges(&self) -> impl Iterator<Item = &FlowChannel>;

            pub fn edge_indices(&self) -> impl Iterator<Item = EdgeIndex>;
        }

        to self.topology.links {
            #[call(iter)]
            pub fn links(&self) -> impl Iterator<Item = &Link>;
        }

        to self.clusters {
            #[call(len)]
            pub fn nr_clusters(&self) -> usize;
        }
    }
}

impl TraversableNetwork<FlowChannel> for SimNetwork {
    fn topology(&self) -> &Topology<FlowChannel> {
        &self.topology
    }

    fn routes(&self) -> &Routes {
        &self.routes
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SimNetworkError {
    #[error("Failed to simulate link")]
    LinkSim(#[from] LinkSimError),

    #[error("Failed to construct empirical distribution")]
    EDist(#[from] EDistError),
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct DelayNetwork {
    topology: Topology<EDistChannel>,
    routes: Routes,
}

impl DelayNetwork {
    pub fn predict<R>(
        &self,
        size: Bytes,
        (src, dst): (NodeId, NodeId),
        mut rng: R,
    ) -> Option<Nanosecs>
    where
        R: Rng,
    {
        let edges = self
            .edge_indices_between(src, dst, |choices| choices.choose(&mut rng))
            .collect::<Vec<_>>();
        if edges.is_empty() {
            return None;
        }
        edges
            .iter()
            .map(|&e| {
                let chan = &self.topology.graph[e];
                chan.dists.for_size(size).map(|dist| {
                    let pktnorm_delay = dist.sample(&mut rng);
                    let nr_pkts = (size.into_f64() / PKTSIZE_MAX.into_f64()).ceil();
                    let delay = nr_pkts * pktnorm_delay;
                    Nanosecs::new(delay as u64)
                })
            })
            .sum()
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

impl TraversableNetwork<EDistChannel> for DelayNetwork {
    fn topology(&self) -> &Topology<EDistChannel> {
        &self.topology
    }

    fn routes(&self) -> &Routes {
        &self.routes
    }
}

trait TraversableNetwork<C: Clone> {
    fn topology(&self) -> &Topology<C>;

    fn routes(&self) -> &Routes;

    fn nr_edges(&self) -> usize {
        self.topology().nr_edges()
    }

    fn edge_indices_between(
        &self,
        src: NodeId,
        dst: NodeId,
        mut choose: impl FnMut(&[NodeId]) -> Option<&NodeId>,
    ) -> std::vec::IntoIter<EdgeIndex> {
        let mut acc = Vec::new();
        let mut cur = src;
        while cur != dst {
            let next_hop_choices = self.routes().next_hops_unchecked(cur, dst);
            match choose(next_hop_choices) {
                Some(&next_hop) => {
                    // These indices are all guaranteed to exist because we have a valid topology
                    let i = *self.topology().idx_of(&cur).unwrap();
                    let j = *self.topology().idx_of(&next_hop).unwrap();
                    let e = self.topology().find_edge(i, j).unwrap();
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

#[cfg(test)]
mod tests {
    use anyhow::Context;

    use crate::network::FlowId;
    use crate::testing;
    use crate::units::{Bytes, Nanosecs};

    use super::*;

    fn find_edge<C: Clone>(topo: &Topology<C>, src: NodeId, dst: NodeId) -> Option<EdgeIndex> {
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
                id: FlowId::new(i),
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
        - 55
        - 45
        "###);

        Ok(())
    }

    #[test]
    fn default_clustering_is_one_to_one() -> anyhow::Result<()> {
        let (nodes, links) = testing::eight_node_config();
        let network = Network::new(&nodes, &links).context("failed to create topology")?;
        let network = network.into_simulations(Vec::new());
        assert_eq!(network.nr_clusters(), network.nr_edges());
        assert!(network
            .clusters
            .iter()
            .enumerate()
            .all(|(i, c)| c.representative().index() == i));
        Ok(())
    }
}
