//! This module defines the various network types that make up `Parsimon`'s core structure.
//!
//! The first step is to construct a [`Network`] of nodes and links. This network is then populated
//! with [flows](Flow) to produce a [`SimNetwork`], where each edge can be independently simulated.
//! Finally, the simulations are run to produce a [`DelayNetwork`], which can be queried for FCT
//! delay estimates.

pub mod topology;
pub mod types;

use std::{collections::HashMap, net::SocketAddr};
use std::net::Ipv4Addr;

use itertools::Itertools;
use petgraph::graph::NodeIndex;
use rand::prelude::*;
use rayon::prelude::*;

pub use petgraph::graph::EdgeIndex;
use rustc_hash::{FxHashMap, FxHashSet};
pub use topology::TopologyError;
pub use types::*;

use crate::{
    cluster::{Cluster, ClusteringAlgo},
    constants::SZ_PKTMAX,
    distribute::{self, WorkerParams},
    edist::EDistError,
    linksim::{
        LinkSim, LinkSimDesc, LinkSimError, LinkSimLink, LinkSimNode, LinkSimNodeKind, LinkSimSpec,
    },
    opts::SimOpts,
    routing::{BfsRoutes, RoutingAlgo},
    units::{BitsPerSec, Bytes, Nanosecs},
    utils,
};

use self::{topology::Topology, types::EDistChannel};

/// A `Network` is a collection of nodes, links, and routes.
#[derive(Debug, Clone)]
pub struct Network<R = BfsRoutes> {
    topology: Topology<BasicChannel>,
    routes: R,
}

impl Network<BfsRoutes> {
    /// Creates a new network with default [BFS routing](`BfsRoutes`).
    pub fn new(nodes: &[Node], links: &[Link]) -> Result<Self, TopologyError> {
        let topology = Topology::new(nodes, links)?;
        let routes = BfsRoutes::new(&topology);
        Ok(Self { topology, routes })
    }
}

impl<R> Network<R>
where
    R: RoutingAlgo + Sync,
{
    /// Creates a new network with a given routing implementation.
    pub fn new_with_routes(
        nodes: &[Node],
        links: &[Link],
        routes: R,
    ) -> Result<Self, TopologyError> {
        let topology = Topology::new(nodes, links)?;
        Ok(Self { topology, routes })
    }

    /// Creates a `SimNetwork`.
    ///
    /// PRECONDITIONS: For each flow in `flows`, `flow.src` and `flow.dst` must be valid hosts in
    /// `network`, and there must be a path between them.
    /// POSTCONDITION: The flows populating each link will be sorted by start time.
    pub fn into_simulations(self, flows: Vec<Flow>) -> SimNetwork<R> {
        let mut topology = Topology::new_traced(&self.topology);
        let assignments = utils::par_chunks(&flows, |flows| {
            let mut assignments = Vec::new();
            for &f @ Flow { id, src, dst, .. } in flows {
                let hash = utils::calculate_hash(&id);
                let path =
                    self.edge_indices_between(src, dst, |choices| {
                        assert!(!choices.is_empty(), "missing path from {src} to {dst}");
                        let idx = hash as usize % choices.len();
                        Some(&choices[idx])
                    });
                for eidx in path {
                    assignments.push((eidx, f));
                }
            }
            assignments
        })
        .fold(
            FxHashMap::default(),
            |mut map: FxHashMap<_, Vec<_>>, (e, f)| {
                map.entry(e).or_default().push(f);
                map
            },
        );
        let assignments = assignments
            .into_par_iter()
            .map(|(eidx, mut flows)| {
                let mut chan = FlowChannel::new_from(&self.topology.graph[eidx]);
                // POSTCONDITION: The flows populating each link will be sorted by start time.
                flows.sort_by_key(|f| f.start);
                for f in flows {
                    chan.push_flow(&f);
                }
                (eidx, chan)
            })
            .collect::<Vec<_>>();
        for (eidx, chan) in assignments {
            topology.graph[eidx] = chan;
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
            channel_to_flowid_map: None,
            path_to_flowid_map: None,
        }
    }
    /// Creates a `SimNetwork` for path.
    pub fn into_simulations_path(self, flows: Vec<Flow>) -> SimNetwork<R> {
        let topology = Topology::new_traced(&self.topology);
        let node_num = topology.graph.node_count();
        // println!("node_num: {:?}", node_num);
        let mut server_address = vec![Ipv4Addr::UNSPECIFIED; node_num as usize];

        for node in topology.graph.node_indices() {
            let node = &topology.graph[node];
            if let NodeKind::Host = node.kind {
                let node_id:usize = node.id.as_usize();
                server_address[node_id] = utils::node_id_to_ip(node_id);
            }
        }
        // println!("server_address: {:?}", server_address);

        // Maintain port number for each host
        let mut port_number = vec![vec![10000; node_num]; node_num];
        // Calculate port numbers in advance
        let mut port_number_map: FxHashMap<usize, u16> = FxHashMap::default();

        for &f in &flows {
            let ids = f.get_ids();
            let sport: u16 = port_number[ids[1]][ids[2]];
            port_number[ids[1]][ids[2]] += 1;
            port_number_map.entry(ids[0]).or_insert(sport);
        }

        let (assignments_0, assignments_1) = utils::par_chunks(&flows, |flows| {
            let mut assignments = Vec::new();
            for &f @ Flow { id, src, dst, .. } in flows {
                // Get the ids (i.e., ID, SRC, DST) as usize of the flow
                let ids = f.get_ids(); 

                let sip= server_address[ids[1]];
                let sip_bytes = sip.octets();
               
                let dip= server_address[ids[2]];
                let dip_bytes = dip.octets();
               
                let sport: u16 = port_number_map[&ids[0]];

                // Create buffer and populate with sip, dip, and ports
                let mut buf = [0u8; 12]; // 4 (sip) + 4 (dip) + 2 (sport) + 2 (dport)

                buf[0..4].copy_from_slice(&sip_bytes);
                buf[0..4].reverse();
                buf[4..8].copy_from_slice(&dip_bytes);
                buf[4..8].reverse();

                // Set ports based on your logic
                let port_combined = (sport as u32) | ((100 as u32) << 16);
                let port_bytes = port_combined.to_be_bytes();
                buf[8..12].copy_from_slice(&port_bytes);
                buf[8..12].reverse();

                let path = self.edge_indices_between_ns3(src, dst, &buf);

                let mut path_vec = vec![(src, dst)];
                for eidx in path {
                    path_vec.push((self.topology.graph[eidx].src(), self.topology.graph[eidx].dst()));
                } 
                for pair in path_vec.iter().skip(1).cloned() {
                    assignments.push((0, pair, vec![pair], id));
                }
                assignments.push((1, (src, dst), path_vec, id));
            }    
            assignments
        })
        .fold(
            (FxHashMap::default(), FxHashMap::default()),
            |(mut map_0, mut map_1): (FxHashMap<(NodeId, NodeId), FxHashSet<FlowId>>,FxHashMap<Vec<(NodeId, NodeId)>, FxHashSet<FlowId>>), (tag, c, p, f)| {
                if tag == 0 {
                    map_0.entry(c).or_default().insert(f);
                } else {
                    map_1.entry(p).or_default().insert(f);
                }
                (map_0, map_1)
            },
        );

        // println!("assignments: {:?}", assignments.len());
        let channel_to_flowid_map = if assignments_0.is_empty() {
            None
        } else {
            Some(assignments_0)
        };

        let path_to_flowid_map = if assignments_1.is_empty() {
            None
        } else {
            Some(assignments_1)
        };

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
            channel_to_flowid_map: channel_to_flowid_map,
            path_to_flowid_map: path_to_flowid_map,
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

    /// Returns the topology of the network.
    pub fn topology(&self) -> &Topology<BasicChannel> {
        &self.topology
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

impl<R> TraversableNetwork<BasicChannel, R> for Network<R>
where
    R: RoutingAlgo,
{
    fn topology(&self) -> &Topology<BasicChannel> {
        &self.topology
    }

    fn routes(&self) -> &R {
        &self.routes
    }
}

/// A `SimNetwork` is similar to a [`Network`], except each link is augmented with a sequence of
/// flows traversing it. These links can be simulated to produce a [`DelayNetwork`]. Optionally,
/// they can also be clustered to reduce the number of simulations.
#[derive(Debug, Clone)]
pub struct SimNetwork<R = BfsRoutes> {
    topology: Topology<FlowChannel>,
    routes: R,

    // Channel clustering
    clusters: Vec<Cluster>,
    // Each channel references these flows by ID
    flows: HashMap<FlowId, Flow>,

    channel_to_flowid_map: Option<FxHashMap<(NodeId, NodeId), FxHashSet<FlowId>>>,
    path_to_flowid_map: Option<FxHashMap<Vec<(NodeId, NodeId)>, FxHashSet<FlowId>>>,
}

impl<R> SimNetwork<R>
where
    R: RoutingAlgo + Sync,
{
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
    pub fn into_delays<S>(self, opts: SimOpts<S>) -> Result<DelayNetwork<R>, SimNetworkError>
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
                let data = match eidx2data.get(&representative) {
                    Some(data) => &data[..],
                    None => &[],
                };
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

    fn simulate_clusters_locally<S>(
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

    fn simulate_clusters<S>(
        &self,
        sim: S,
        workers: &[SocketAddr],
    ) -> Result<HashMap<EdgeIndex, Vec<FctRecord>>, SimNetworkError>
    where
        S: LinkSim + Sync,
    {
        let sim = (sim.name(), serde_json::to_string(&sim)?);
        let assignments = self.assign_work_randomly(workers);
        let assignments = assignments
            .iter()
            .par_bridge()
            .map(|(worker, edges)| {
                let descs = edges
                    .par_iter()
                    .filter_map(|&edge| self.link_sim_desc(edge))
                    .collect::<Vec<_>>();
                let flows = descs
                    .iter()
                    .flat_map(|d| d.flows.iter())
                    .collect::<FxHashSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                let flows =
                    utils::par_chunks(&flows, |flows| {
                        flows
                            .iter()
                            .map(|&id| self.flows.get(id).unwrap().to_owned())
                            .collect()
                    })
                    .collect();
                let params = WorkerParams {
                    link_sim: sim.clone(),
                    descs,
                    flows,
                };
                (worker, params)
            })
            .collect::<Vec<_>>();
        let rt = tokio::runtime::Runtime::new()?;
        let results = rt.block_on(async {
            let handles = assignments
                .into_iter()
                .map(|(&worker, params)| tokio::spawn(distribute::work_remote(worker, params)))
                .collect::<Vec<_>>();
            let mut results = Vec::new();
            for handle in handles {
                results.append(&mut handle.await??);
            }
            Result::<_, SimNetworkError>::Ok(results)
        })?;
        Ok(results
            .into_iter()
            .map(|(edge, records)| (EdgeIndex::new(edge), records))
            .collect())
    }

    fn assign_work_randomly(&self, workers: &[SocketAddr]) -> Vec<(SocketAddr, Vec<EdgeIndex>)> {
        assert!(!workers.is_empty());
        let mut edges = self
            .clusters
            .iter()
            .map(|c| c.representative())
            .collect::<Vec<_>>();
        let mut rng = StdRng::seed_from_u64(0);
        edges.shuffle(&mut rng);
        let chunk_size = edges.len() / workers.len();
        workers
            .iter()
            .zip(edges.chunks(chunk_size))
            .map(|(&w, es)| (w, es.to_vec()))
            .collect()
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

    /// get path_to_flowid_map
    pub fn get_routes(&self) -> Option<(&FxHashMap<(NodeId, NodeId), FxHashSet<FlowId>>, &FxHashMap<Vec<(NodeId, NodeId)>, FxHashSet<FlowId>>)> {
        self.channel_to_flowid_map.as_ref().and_then(|channel_map| {
            self.path_to_flowid_map.as_ref().map(|path_map| (channel_map, path_map))
        })
    }

    /// Returns a path from `src` to `dst`, using `choose` to select a path when there are multiple
    /// options.
    pub fn path(
        &self,
        src: NodeId,
        dst: NodeId,
        choose: impl FnMut(&[NodeId]) -> Option<&NodeId>,
    ) -> Path<FlowChannel> {
        <Self as TraversableNetwork<FlowChannel, R>>::path(self, src, dst, choose)
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
        let duration = self.duration_of(eidx)?;
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
        let chan = self.edge(eidx)?;
        Some(chan.duration())
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
            .chain([&bsrc, &bdst])
            .unique()
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

impl<R> TraversableNetwork<FlowChannel, R> for SimNetwork<R>
where
    R: RoutingAlgo,
{
    fn topology(&self) -> &Topology<FlowChannel> {
        &self.topology
    }

    fn routes(&self) -> &R {
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
    TokioIo(#[from] tokio::io::Error),

    /// Tokio join error.
    #[error("Tokio join error.")]
    TokioJoin(#[from] tokio::task::JoinError),
}

/// A `DelayNetwork` is a network in which all edges contain empirical distributions of FCT delay
/// bucketed by flow size.
#[derive(Debug, Clone)]
#[allow(unused)]
pub struct DelayNetwork<R = BfsRoutes> {
    topology: Topology<EDistChannel>,
    routes: R,
}

impl<R> DelayNetwork<R>
where
    R: RoutingAlgo,
{
    /// Predict a point estimate of delay for a flow of a particular `size` going from `src` to
    /// `dst`.
    pub fn predict<RNG>(
        &self,
        size: Bytes,
        (src, dst): (NodeId, NodeId),
        mut rng: RNG,
    ) -> Option<Nanosecs>
    where
        RNG: Rng,
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
    pub fn ideal_fct<RNG>(
        &self,
        size: Bytes,
        (src, dst): (NodeId, NodeId),
        mut rng: RNG,
    ) -> Option<Nanosecs>
    where
        RNG: Rng,
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
    pub fn slowdown<RNG>(
        &self,
        size: Bytes,
        (src, dst): (NodeId, NodeId),
        mut rng: RNG,
    ) -> Option<f64>
    where
        RNG: Rng,
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

impl<R> TraversableNetwork<EDistChannel, R> for DelayNetwork<R>
where
    R: RoutingAlgo,
{
    fn topology(&self) -> &Topology<EDistChannel> {
        &self.topology
    }

    fn routes(&self) -> &R {
        &self.routes
    }
}

pub(crate) trait TraversableNetwork<C: Clone + Channel, R: RoutingAlgo> {
    fn topology(&self) -> &Topology<C>;

    fn routes(&self) -> &R;

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
            let next_hop_choices =
                match self.routes().next_hops(cur, dst) {
                    Some(hops) => hops,
                    None => break,
                };
            match choose(&next_hop_choices) {
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

    fn edge_indices_between_ns3(
        &self,
        src: NodeId,
        dst: NodeId,
        buf: &[u8],
    ) -> Vec<EdgeIndex> {
        let mut acc = Vec::new();
        let mut cur = src;
        while cur != dst {
            let next_hop_choices = match self.routes().next_hops(cur, dst) {
                Some(hops) => hops,
                None => break,
            };
            
            let idx = if let NodeKind::Switch = self.topology().graph[*self.topology().idx_of(&cur).unwrap()].kind {
                let hash = utils::calculate_hash_ns3(buf, buf.len(), cur.as_usize() as u32);
                let tmp=(hash % next_hop_choices.len() as u32) as usize;
                // Print input parameters
                // println!("next_hop_choices: {:?}\nKey: {:?}\nLength of key: {}\nSeed value: {}\nIndex: {}", next_hop_choices,buf,buf.len(),cur,tmp);
                tmp
            } else {
                0 // For host nodes, always choose the first next hop
            };
            
            let next_hop = next_hop_choices[idx];
    
            // These indices are all guaranteed to exist because we have a valid topology
            let i = *self.topology().idx_of(&cur).unwrap();
            let j = *self.topology().idx_of(&next_hop).unwrap();
            let e = self.topology().find_edge(i, j).unwrap();
            acc.push(e);
    
            cur = next_hop;
        }
        acc
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
        let flows = (0..1)
            .map(|i| Flow {
                id: FlowId::new(i),
                src: NodeId::new(0),
                dst: NodeId::new(3),
                size: Bytes::new(1_000),
                start: Nanosecs::new(2_000_000_000),
            })
            .collect::<Vec<_>>();
        let network = network.into_simulations_path(flows);

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
}

// #[cfg(test)]
// mod tests {
//     use std::collections::BTreeMap;

//     use anyhow::Context;

//     use crate::network::FlowId;
//     use crate::testing;
//     use crate::units::{Bytes, Nanosecs};

//     use super::*;

//     fn find_edge<C: Clone>(topo: &Topology<C>, src: NodeId, dst: NodeId) -> Option<EdgeIndex> {
//         topo.idx_of(&src)
//             .and_then(|&a| topo.idx_of(&dst).map(|&b| (a, b)))
//             .and_then(|(a, b)| topo.graph.find_edge(a, b))
//     }

//     // This test creates an eight-node topology and sends some flows with the
//     // same source and destination across racks. All flows will traverse
//     // exactly one ECMP group in the upwards direction. While we don't know
//     // exactly how many flows should traverse each link in the group, we can
//     // check that the final counts are close to equal. If a different hashing
//     // algorithm is used, the exact counts will change, and this test will need
//     // to be updated to use the new snapshot.
//     #[test]
//     fn ecmp_replication_works() -> anyhow::Result<()> {
//         let (nodes, links) = testing::eight_node_config();
//         let network = Network::new(&nodes, &links).context("failed to create topology")?;
//         let flows = (0..100)
//             .map(|i| Flow {
//                 id: FlowId::new(i),
//                 src: NodeId::new(0),
//                 dst: NodeId::new(3),
//                 size: Bytes::ZERO,
//                 start: Nanosecs::ZERO,
//             })
//             .collect::<Vec<_>>();
//         let network = network.into_simulations(flows);

//         // The ECMP group contains edges (4, 6) and (4, 7)
//         let e1 = find_edge(&network.topology, NodeId::new(4), NodeId::new(6)).unwrap();
//         let e2 = find_edge(&network.topology, NodeId::new(4), NodeId::new(7)).unwrap();

//         // Flow counts for the links should be close to each other
//         insta::assert_yaml_snapshot!((
//             network.topology.graph[e1].flows.len(),
//             network.topology.graph[e2].flows.len(),
//         ), @r###"
//         ---
//         - 55
//         - 45
//         "###);

//         Ok(())
//     }

//     #[test]
//     fn default_clustering_is_one_to_one() -> anyhow::Result<()> {
//         let (nodes, links) = testing::eight_node_config();
//         let network = Network::new(&nodes, &links).context("failed to create topology")?;
//         let network = network.into_simulations(Vec::new());
//         assert_eq!(network.nr_clusters(), network.nr_edges());
//         assert!(network
//             .clusters
//             .iter()
//             .enumerate()
//             .all(|(i, c)| c.representative().index() == i));
//         Ok(())
//     }

//     #[test]
//     fn link_sim_desc_correct() -> anyhow::Result<()> {
//         let (nodes, links) = testing::eight_node_config();
//         let flows = vec![
//             Flow {
//                 id: FlowId::new(0),
//                 src: NodeId::new(0),
//                 dst: NodeId::new(1),
//                 size: Bytes::new(1234),
//                 start: Nanosecs::new(1_000_000_000),
//             },
//             Flow {
//                 id: FlowId::new(1),
//                 src: NodeId::new(0),
//                 dst: NodeId::new(2),
//                 size: Bytes::new(5678),
//                 start: Nanosecs::new(2_000_000_000),
//             },
//         ];

//         let network = Network::new(&nodes, &links)?;
//         let network = network.into_simulations(flows);
//         let check = network
//             .edge_indices()
//             .filter_map(|eidx| {
//                 let chan = network.edge(eidx).unwrap();
//                 let desc = network.link_sim_desc(eidx)?;
//                 Some(((chan.src(), chan.dst()), desc))
//             })
//             .collect::<BTreeMap<_, _>>();

//         insta::assert_yaml_snapshot!(check);

//         Ok(())
//     }
// }
