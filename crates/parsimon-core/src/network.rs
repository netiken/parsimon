//! This module defines the various network types that make up `Parsimon`'s core structure.
//!
//! The first step is to construct a [`Network`] of nodes and links. This network is then populated
//! with [flows](Flow) to produce a [`SimNetwork`], where each edge can be independently simulated.
//! Finally, the simulations are run to produce a [`DelayNetwork`], which can be queried for FCT
//! delay estimates.

pub(crate) mod routing;
pub(crate) mod topology;
pub mod types;

use std::{collections::HashMap, net::SocketAddr};

use chrono::Utc;
use itertools::Itertools;
use petgraph::graph::NodeIndex;
use rand::prelude::*;
use rayon::prelude::*;

pub use petgraph::graph::EdgeIndex;
use rustc_hash::FxHashSet;
pub use topology::TopologyError;
pub use types::*;

use crate::{
    cluster::{Cluster, ClusteringAlgo},
    constants::SZ_PKTMAX,
    distribute::{self, WorkerChunk, WorkerParams},
    edist::EDistError,
    linksim::{
        LinkSim, LinkSimDesc, LinkSimError, LinkSimLink, LinkSimNode, LinkSimNodeKind, LinkSimSpec,
    },
    opts::SimOpts,
    units::{BitsPerSec, Bytes, Nanosecs},
    utils,
};

use self::{
    routing::Routes,
    topology::Topology,
    types::{BasicChannel, EDistChannel, FlowChannel, Link, Node},
};

/// A `Network` is a collection of nodes, links, and routes.
#[derive(Debug, Clone)]
pub struct Network {
    topology: Topology<BasicChannel>,
    routes: Routes,
}

impl Network {
    /// Creates a new network.
    pub fn new(nodes: &[Node], links: &[Link]) -> Result<Self, TopologyError> {
        let topology = Topology::new(nodes, links)?;
        let routes = Routes::new(&topology);
        Ok(Self { topology, routes })
    }

    /// Creates a `SimNetwork`.
    ///
    /// PRECONDITIONS: For each flow in `flows`, `flow.src` and `flow.dst` must be valid hosts in
    /// `network`, and there must be a path between them.
    /// POSTCONDITION: The flows populating each link will be sorted by start time.
    pub fn into_simulations(self, flows: Vec<Flow>) -> SimNetwork {
        let mut topology = Topology::new_traced(&self.topology);
        let assignments = utils::par_chunks(&flows, |flows| {
            let mut assignments = Vec::new();
            for &f @ Flow { id, src, dst, .. } in flows {
                let hash = utils::calculate_hash(&id);
                let path = self.edge_indices_between(src, dst, |choices| {
                    assert!(!choices.is_empty(), "missing path from {src} to {dst}");
                    let idx = hash as usize % choices.len();
                    Some(&choices[idx])
                });
                for eidx in path {
                    assignments.push((eidx, f));
                }
            }
            assignments
        });
        // POSTCONDITION: The flows populating each link will be sorted by start time.
        for (eidx, flow) in assignments.sorted_by_key(|&(_, flow)| flow.start) {
            topology.graph[eidx].push_flow(&flow);
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
            routes: self.routes,
            clusters,
            flows: flows.into_iter().map(|f| (f.id, f)).collect(),
        }
    }

    /// Returns the [NodeId]s of all hosts in the network.
    pub fn host_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes().filter_map(|n| match n.kind {
            NodeKind::Host => Some(n.id),
            NodeKind::Switch => None,
        })
    }

    /// Returns all nodes directly connected to the node with the given ID.
    pub fn neighbors(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let idx = self
            .topology
            .idx_of(&id)
            .copied()
            .unwrap_or_else(|| NodeIndex::new(usize::MAX));
        self.topology
            .graph
            .neighbors(idx)
            .map(|idx| self.topology.graph[idx].id)
    }

    delegate::delegate! {
        to self.topology.graph {
            /// Returns an iterator over all nodes in the network.
            #[call(node_weights)]
            pub fn nodes(&self) -> impl Iterator<Item = &Node>;
        }

        to self.topology.links {
            /// Returns an iterator over all links in the network.
            #[call(iter)]
            pub fn links(&self) -> impl Iterator<Item = &Link>;
        }
    }
}

impl TraversableNetwork<BasicChannel> for Network {
    fn topology(&self) -> &Topology<BasicChannel> {
        &self.topology
    }

    fn routes(&self) -> &Routes {
        &self.routes
    }
}

/// A `SimNetwork` is similar to a [`Network`], except each link is augmented with a sequence of
/// flows traversing it. These links can be simulated to produce a [`DelayNetwork`]. Optionally,
/// they can also be clustered to reduce the number of simulations.
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
    /// Clusters the links in the network with the given clustering algorithm.
    pub fn cluster<C>(&mut self, algorithm: C)
    where
        C: ClusteringAlgo,
    {
        let clusters = algorithm.cluster(self);
        self.clusters = clusters;
    }

    /// Converts the `SimNetwork` into a [`DelayNetwork`] by performing link simulations and
    /// processing the results into empirical distributions bucketed by flow size.
    pub fn into_delays<S>(self, opts: SimOpts<S>) -> Result<DelayNetwork, SimNetworkError>
    where
        S: LinkSim + Sync,
    {
        let mut topology = Topology::new_edist(&self.topology);

        let eidx2data = if opts.is_local() {
            self.simulate_clusters_locally(opts.link_sim)?
        } else {
            self.simulate_clusters(opts.link_sim, &opts.workers)?
        };

        // Every channel gets filled with delay distributions. All channels in the same cluster get
        // filled using the cluster representative's data.
        for cluster in &self.clusters {
            let representative = cluster.representative();
            for &member in cluster.members() {
                // Fill channel with packet-normalized delay predictions
                let data = eidx2data.get(&representative).unwrap();
                if !data.is_empty() {
                    topology.graph[member].dists.fill(
                        data,
                        |rec| rec.size,
                        |rec| rec.pktnorm_delay(),
                        opts.bucket_opts,
                    )?;
                }
            }
        }
        Ok(DelayNetwork {
            topology,
            routes: self.routes,
        })
    }

    pub(crate) fn simulate_clusters_locally<S>(
        &self,
        sim: S,
    ) -> Result<HashMap<EdgeIndex, Vec<FctRecord>>, SimNetworkError>
    where
        S: LinkSim + Sync,
    {
        let (s, r) = crossbeam_channel::unbounded();
        // Simulate all cluster representatives in parallel.
        self.clusters.par_iter().try_for_each_with(s, |s, c| {
            let edge = c.representative();
            let data = match self.link_sim_desc(edge) {
                Some(desc) => {
                    let flows = desc
                        .flows
                        .iter()
                        .map(|id| self.flows.get(id).unwrap().to_owned())
                        .collect::<Vec<_>>();
                    let spec = LinkSimSpec {
                        edge: desc.edge,
                        bottleneck: desc.bottleneck,
                        other_links: desc.other_links,
                        nodes: desc.nodes,
                        flows,
                    };
                    sim.simulate(spec)?
                }
                None => Vec::new(),
            };
            s.send((edge, data)).unwrap(); // the channel should never become disconnected
            Result::<(), SimNetworkError>::Ok(())
        })?;
        Ok(r.iter().collect())
    }

    pub(crate) fn simulate_clusters<S>(
        &self,
        sim: S,
        workers: &[SocketAddr],
    ) -> Result<HashMap<EdgeIndex, Vec<FctRecord>>, SimNetworkError>
    where
        S: LinkSim + Sync,
    {
        let chunk_size = self.clusters.len() / workers.len();
        let sim = (sim.name(), serde_json::to_string(&sim)?);
        let assignments =
            workers
                .iter()
                .zip(self.clusters.chunks(chunk_size))
                .map(|(&worker, clusters)| {
                    let descs = clusters
                        .iter()
                        .filter_map(|c| self.link_sim_desc(c.representative()))
                        .collect::<Vec<_>>();
                    let flows = descs
                        .iter()
                        .flat_map(|d| d.flows.iter())
                        .unique()
                        .map(|id| self.flows.get(id).unwrap().to_owned())
                        .collect::<Vec<_>>();
                    let chunk = WorkerChunk { descs, flows };
                    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                    let params = WorkerParams {
                        link_sim: sim.clone(),
                        chunk_path: format!("/tmp/pmn_input_{}.txt", timestamp).into(),
                    };
                    (worker, chunk, params)
                });
        let rt = tokio::runtime::Runtime::new()?;
        let results = rt.block_on(async {
            let mut results = Vec::new();
            for (worker, chunk, params) in assignments {
                let mut r = distribute::work_remote(worker, params, chunk).await?;
                results.append(&mut r);
            }
            Result::<_, SimNetworkError>::Ok(results)
        })?;
        Ok(results
            .into_iter()
            .map(|(edge, records)| (EdgeIndex::new(edge), records))
            .collect())
    }

    /// Returns the flows traversing a given edge, or `None` if the edge doesn't exist.
    pub fn flows_on(&self, edge: EdgeIndex) -> Option<Vec<Flow>> {
        self.edge(edge).map(|chan| {
            chan.flow_ids()
                .map(|id| self.flows.get(&id).unwrap().to_owned().to_owned())
                .collect()
        })
    }

    /// Returns the node with the given ID, or `None` if no such node exists.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.topology
            .idx_of(&id)
            .map(|&idx| &self.topology.graph[idx])
    }

    /// Returns the edge connecting two nodes, if any.
    pub fn find_edge(&self, a: NodeId, b: NodeId) -> Option<EdgeIndex> {
        let a = *self.topology.idx_of(&a)?;
        let b = *self.topology.idx_of(&b)?;
        self.topology.graph.find_edge(a, b)
    }

    /// Gets a reference to the `SimNetwork`'s clusters.
    pub fn clusters(&self) -> &[Cluster] {
        self.clusters.as_ref()
    }

    /// Sets the `SimNetwork`'s clusters.
    pub fn set_clusters(&mut self, clusters: Vec<Cluster>) {
        self.clusters = clusters;
    }

    /// Returns a path from `src` to `dst`, using `choose` to select a path when there are multiple
    /// options.
    pub fn path(
        &self,
        src: NodeId,
        dst: NodeId,
        choose: impl FnMut(&[NodeId]) -> Option<&NodeId>,
    ) -> Path<FlowChannel> {
        <Self as TraversableNetwork<FlowChannel>>::path(self, src, dst, choose)
    }

    /// Returns an iterator over all link loads.
    pub fn link_loads(&self) -> impl Iterator<Item = f64> + '_ {
        self.edge_indices().filter_map(|eidx| self.load_of(eidx))
    }

    /// Returns the load of a particular link, or `None` if the link doesn't exist.
    pub fn load_of(&self, eidx: EdgeIndex) -> Option<f64> {
        let chan = self.edge(eidx)?;
        let flows = self.flows_on(eidx)?;
        let nr_bytes = flows.iter().map(|f| f.size).sum::<Bytes>();
        let duration = flows.last().map(|f| f.start).unwrap_or_default()
            - flows.first().map(|f| f.start).unwrap_or_default();
        (duration != Nanosecs::ZERO)
            .then(|| {
                assert!(chan.bandwidth() != BitsPerSec::ZERO);
                let bps = nr_bytes.into_f64() * 8.0 * 1e9 / duration.into_f64();
                bps / chan.bandwidth().into_f64()
            })
            .or(Some(0.0))
    }

    /// Returns the rate of the ACKs on a given link, or `None` if the link doesn't exist.
    pub fn ack_rate_of(&self, eidx: EdgeIndex) -> Option<BitsPerSec> {
        let chan = self.edge(eidx)?;
        // TODO: Make finding a reverse edge more efficient
        let reverse_edge = self.find_edge(chan.dst(), chan.src()).unwrap();
        let reverse_chan = self.edge(reverse_edge)?;
        let duration = self.duration_of(reverse_edge)?;
        if duration == Nanosecs::ZERO {
            return Some(BitsPerSec::ZERO);
        }
        let inner = reverse_chan.nr_ack_bytes.into_f64() * 8.0 * 1e9 / duration.into_f64();
        Some(BitsPerSec::new(inner.round() as u64))
    }

    pub(crate) fn duration_of(&self, eidx: EdgeIndex) -> Option<Nanosecs> {
        let flows = self.flows_on(eidx)?;
        let duration = flows.last().map(|f| f.start).unwrap_or_default()
            - flows.first().map(|f| f.start).unwrap_or_default();
        Some(duration)
    }

    /// Returns a link-level descriptor for a given edge.
    pub fn link_sim_desc(&self, edge: EdgeIndex) -> Option<LinkSimDesc> {
        let chan = self.edge(edge)?;
        if chan.nr_flows() == 0 {
            // Sources and destinations for link-level topologies are extracted from flows, so if
            // there are no flows, there is no link-level topology.
            return None;
        }

        // NOTE: `bsrc` and `bdst` may be in `srcs` and `dsts`, respectively
        let (srcs, dsts) = (&chan.flow_srcs, &chan.flow_dsts);
        let (bsrc, bdst) = (chan.src(), chan.dst());

        assert!(srcs.intersection(dsts).count() == 0);
        let nodes = srcs
            .iter()
            .chain(dsts.iter())
            .chain([&bsrc, &bdst].into_iter())
            .collect::<FxHashSet<_>>();
        let nodes = nodes
            .into_iter()
            .map(|&id| {
                let Node { kind, .. } = self.node(id).unwrap();
                let kind = match kind {
                    NodeKind::Switch => LinkSimNodeKind::Switch,
                    NodeKind::Host if srcs.contains(&id) => LinkSimNodeKind::Source,
                    NodeKind::Host if dsts.contains(&id) => LinkSimNodeKind::Destination,
                    _ => unreachable!("`link_sim_desc`: unknown node kind"),
                };
                LinkSimNode { id, kind }
            })
            .collect::<Vec<_>>();

        let mut other_links = Vec::new();
        // Connect sources to the bottleneck. If `bsrc` is in `srcs`, then the
        // bottleneck channel is assumed to be a host-ToR up-channel.
        if srcs.contains(&bsrc) {
            assert!(srcs.len() == 1);
        } else {
            for &src in srcs {
                // CORRECTNESS: assumes all paths from `src` to `bsrc` have the
                // same min bandwidth and delay
                let path = self.path(src, bsrc, |choices| choices.first());
                let &(eidx, chan) = path.iter().next().unwrap();
                let link = LinkSimLink {
                    from: src,
                    to: bsrc,
                    total_bandwidth: chan.bandwidth(),
                    available_bandwidth: chan.bandwidth() - self.ack_rate_of(eidx).unwrap(),
                    delay: path.delay(),
                };
                other_links.push(link);
            }
        }
        // Connect the bottleneck to destinations with _fat links_. If `bdst`
        // is in `dsts`, then the bottleneck channel is assumed to be a
        // ToR-host down-channel.
        if dsts.contains(&bdst) {
            assert!(dsts.len() == 1);
        } else {
            for &dst in dsts {
                // CORRECTNESS: assumes all paths from `bdst` to `dst` have the
                // same min bandwidth and delay
                let path = self.path(bdst, dst, |choices| choices.first());
                let bandwidth = path.bandwidths().min().unwrap().scale_by(10.0);
                let link = LinkSimLink {
                    from: bdst,
                    to: dst,
                    total_bandwidth: bandwidth,
                    available_bandwidth: bandwidth,
                    delay: path.delay(),
                };
                other_links.push(link);
            }
        }
        // Now include the bottleneck channel
        let bottleneck = LinkSimLink {
            from: bsrc,
            to: bdst,
            total_bandwidth: chan.bandwidth(),
            available_bandwidth: chan.bandwidth() - self.ack_rate_of(edge).unwrap(),
            delay: chan.delay(),
        };

        Some(LinkSimDesc {
            edge: edge.index(),
            bottleneck,
            other_links,
            nodes,
            flows: chan.flows.clone(),
        })
    }

    delegate::delegate! {
        to self.topology.graph {
            /// Returns an iterator over all nodes in the network.
            #[call(node_weights)]
            pub fn nodes(&self) -> impl Iterator<Item = &Node>;

            /// Returns the `FlowChannel` at the given index, if any.
            #[call(edge_weight)]
            pub fn edge(&self, idx: EdgeIndex) -> Option<&FlowChannel>;

            /// Returns an iterator over all `FlowChannel`s in the network.
            #[call(edge_weights)]
            pub fn channels(&self) -> impl Iterator<Item = &FlowChannel>;

            /// Returns an iterator over all edge indices.
            pub fn edge_indices(&self) -> impl Iterator<Item = EdgeIndex>;
        }

        to self.topology.links {
            /// Returns an iterator over all `Link`s in the network.
            #[call(iter)]
            pub fn links(&self) -> impl Iterator<Item = &Link>;
        }

        to self.clusters {
            /// Returns the number of clusters in the network. By default, each link is in its own
            /// cluster.
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

/// Errors which can be encountered running link-level simulations.
#[derive(Debug, thiserror::Error)]
pub enum SimNetworkError {
    /// Error simulating link.
    #[error("Failed to simulate link")]
    LinkSim(#[from] LinkSimError),

    /// Error constructing empirical distribution.
    #[error("Failed to construct empirical distribution")]
    EDist(#[from] EDistError),

    /// OpenSSH error.
    #[error("OpenSSH error")]
    OpenSSH(#[from] openssh::Error),

    /// SFTP client error.
    #[error("SFTP client error")]
    Sftp(#[from] openssh_sftp_client::Error),

    /// MessagePack encode error.
    #[error("MessagePack encode error")]
    RmpEncode(#[from] rmp_serde::encode::Error),

    /// MessagePack decode error.
    #[error("MessagePack decode error")]
    RmpDecode(#[from] rmp_serde::decode::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error")]
    Json(#[from] serde_json::Error),

    /// Tokio IO error.
    #[error("Tokio IO error.")]
    Tokio(#[from] tokio::io::Error),
}

/// A `DelayNetwork` is a network in which all edges contain empirical distributions of FCT delay
/// bucketed by flow size.
#[derive(Debug, Clone)]
#[allow(unused)]
pub struct DelayNetwork {
    topology: Topology<EDistChannel>,
    routes: Routes,
}

impl DelayNetwork {
    /// Predict a point estimate of delay for a flow of a particular `size` going from `src` to
    /// `dst`.
    pub fn predict<R>(
        &self,
        size: Bytes,
        (src, dst): (NodeId, NodeId),
        mut rng: R,
    ) -> Option<Nanosecs>
    where
        R: Rng,
    {
        let channels = self
            .edge_indices_between(src, dst, |choices| choices.choose(&mut rng))
            .map(|e| &self.topology.graph[e])
            .collect::<Vec<_>>();
        if channels.is_empty() {
            return None;
        }
        channels
            .iter()
            .map(|&chan| chan.dists.for_size(size).map(|dist| dist.sample(&mut rng)))
            .sum::<Option<f64>>()
            .map(|pktnorm_delay| {
                let nr_pkts = (size.into_f64() / SZ_PKTMAX.into_f64()).ceil();
                let delay = nr_pkts * pktnorm_delay;
                Nanosecs::new(delay as u64)
            })
    }

    /// Compute the ideal FCT on an unloaded network for a flow of `size` bytes going from `src` to
    /// `dst.
    pub fn ideal_fct<R>(
        &self,
        size: Bytes,
        (src, dst): (NodeId, NodeId),
        mut rng: R,
    ) -> Option<Nanosecs>
    where
        R: Rng,
    {
        let channels = self
            .edge_indices_between(src, dst, |choices| choices.choose(&mut rng))
            .map(|e| &self.topology.graph[e])
            .collect::<Vec<_>>();
        if channels.is_empty() {
            return None;
        }
        Some(utils::ideal_fct(size, &channels))
    }

    /// Predict a point estimate of slowdown for a flow of a particular `size` going from `src` to
    /// `dst`.
    ///
    /// Slowdown is defined as the measured FCT divided by the ideal FCT.
    pub fn slowdown<R>(&self, size: Bytes, (src, dst): (NodeId, NodeId), mut rng: R) -> Option<f64>
    where
        R: Rng,
    {
        let channels = self
            .edge_indices_between(src, dst, |choices| choices.choose(&mut rng))
            .map(|e| &self.topology.graph[e])
            .collect::<Vec<_>>();
        if channels.is_empty() {
            return None;
        }
        let ideal_fct = utils::ideal_fct(size, &channels);
        let delay = channels
            .iter()
            .map(|&chan| chan.dists.for_size(size).map(|dist| dist.sample(&mut rng)))
            .sum::<Option<f64>>()
            .map(|pktnorm_delay| {
                let nr_pkts = (size.into_f64() / SZ_PKTMAX.into_f64()).ceil();
                let delay = nr_pkts * pktnorm_delay;
                Nanosecs::new(delay as u64)
            })?;
        let real_fct = ideal_fct + delay;
        Some(real_fct.into_f64() / ideal_fct.into_f64())
    }

    delegate::delegate! {
        to self.topology.graph {
            /// Returns an iterator over the [nodes](Node) in the network.
            #[call(node_weights)]
            pub fn nodes(&self) -> impl Iterator<Item = &Node>;
        }

        to self.topology.links {
            /// Returns an iterator over the [links](Link) in the network.
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

pub(crate) trait TraversableNetwork<C: Clone + Channel> {
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

    fn path(
        &self,
        src: NodeId,
        dst: NodeId,
        choose: impl FnMut(&[NodeId]) -> Option<&NodeId>,
    ) -> Path<C> {
        let channels = self
            .edge_indices_between(src, dst, choose)
            .map(|eidx| (eidx, &self.topology().graph[eidx]))
            .collect();
        Path::new(channels)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

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

    #[test]
    fn link_sim_desc_correct() -> anyhow::Result<()> {
        let (nodes, links) = testing::eight_node_config();
        let flows = vec![
            Flow {
                id: FlowId::new(0),
                src: NodeId::new(0),
                dst: NodeId::new(1),
                size: Bytes::new(1234),
                start: Nanosecs::new(1_000_000_000),
            },
            Flow {
                id: FlowId::new(1),
                src: NodeId::new(0),
                dst: NodeId::new(2),
                size: Bytes::new(5678),
                start: Nanosecs::new(2_000_000_000),
            },
        ];

        let network = Network::new(&nodes, &links)?;
        let network = network.into_simulations(flows);
        let check = network
            .edge_indices()
            .filter_map(|eidx| {
                let chan = network.edge(eidx).unwrap();
                let desc = network.link_sim_desc(eidx)?;
                Some(((chan.src(), chan.dst()), desc))
            })
            .collect::<BTreeMap<_, _>>();

        insta::assert_yaml_snapshot!(check);

        Ok(())
    }
}
