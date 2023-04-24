//! This module defines the [`LinkSim`] trait that every link simulator must implement as well as
//! related types.

use std::iter;

use petgraph::graph::EdgeIndex;
use rustc_hash::FxHashMap;

use crate::{
    network::{
        routing::Routes,
        topology::Topology,
        types::{BasicChannel, Link, Node},
        FctRecord, Flow, FlowId, NodeId, NodeKind, TopologyError, TraversableNetwork,
    },
    units::{BitsPerSec, Nanosecs},
};

/// The return type of a link simulation.
pub type LinkSimResult = Result<Vec<FctRecord>, LinkSimError>;

/// An interface for link simulators.
pub trait LinkSim {
    /// Given [`LinkSimSpec`], simulate it and return a collection of FCT records.
    fn simulate(&self, spec: LinkSimSpec) -> LinkSimResult;
}

impl<T: LinkSim> LinkSim for &T {
    fn simulate(&self, spec: LinkSimSpec) -> LinkSimResult {
        (*self).simulate(spec)
    }
}

/// A full specification for a link-level simulation.
#[derive(Debug)]
pub struct LinkSimSpec {
    /// The bottleneck.
    pub bottleneck: LinkSimLink,
    /// The links other than the bottleneck.
    pub other_links: Vec<LinkSimLink>,
    /// The nodes.
    pub nodes: Vec<LinkSimNode>,
    /// The flows.
    pub flows: Vec<Flow>,
}

impl LinkSimSpec {
    /// Returns the nodes in the spec, erasing any `LinkSim`-specific information.
    pub fn generic_nodes(&self) -> impl Iterator<Item = Node> + '_ {
        self.nodes.iter().map(|n| {
            let kind = match n.kind {
                LinkSimNodeKind::Source | LinkSimNodeKind::Destination => NodeKind::Host,
                LinkSimNodeKind::Switch => NodeKind::Switch,
            };
            Node::new(n.id, kind)
        })
    }

    /// Returns the links in the spec, erasing any `LinkSim`-specific information. Bandwidths are
    /// translated using the `available_bandwidth` field of `LinkSimLink`.
    pub fn generic_links(&self) -> impl Iterator<Item = Link> + '_ {
        iter::once(&self.bottleneck)
            .chain(self.other_links.iter())
            .map(|l| Link::new(l.a, l.b, l.available_bandwidth, l.delay))
    }

    /// Creates a copy of a `LinkSimSpec` in which all node IDs are contiguous and returns the
    /// `NodeId` mappings.
    pub fn contiguousify(&self) -> (Self, FxHashMap<NodeId, NodeId>) {
        let old2new = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id, NodeId::new(i)))
            .collect::<FxHashMap<_, _>>();
        (
            Self {
                bottleneck: LinkSimLink {
                    a: *old2new.get(&self.bottleneck.a).unwrap(),
                    b: *old2new.get(&self.bottleneck.b).unwrap(),
                    ..self.bottleneck
                },
                other_links: self
                    .other_links
                    .iter()
                    .map(|&l| LinkSimLink {
                        a: *old2new.get(&l.a).unwrap(),
                        b: *old2new.get(&l.b).unwrap(),
                        ..l
                    })
                    .collect::<Vec<_>>(),
                nodes: self
                    .nodes
                    .iter()
                    .map(|&n| LinkSimNode {
                        id: *old2new.get(&n.id).unwrap(),
                        ..n
                    })
                    .collect::<Vec<_>>(),
                flows: self
                    .flows
                    .iter()
                    .map(|&f| Flow {
                        src: *old2new.get(&f.src).unwrap(),
                        dst: *old2new.get(&f.dst).unwrap(),
                        ..f
                    })
                    .collect::<Vec<_>>(),
            },
            old2new,
        )
    }

    /// Returns a [`LinkSimTopo`] corresponding to this link-level simulation.
    pub fn topo(&self) -> Result<LinkSimTopo, TopologyError> {
        LinkSimTopo::new(self)
    }
}

/// Link-level topology
#[derive(Debug)]
pub struct LinkSimTopo {
    /// The link-level topology
    topology: Topology<BasicChannel>,
    /// The routing table
    routes: Routes,
    /// Mapping of external `NodeId`s to internal ones.
    old2new: FxHashMap<NodeId, NodeId>,
}

impl LinkSimTopo {
    /// Creates a new link level topology.
    pub fn new(spec: &LinkSimSpec) -> Result<Self, TopologyError> {
        let (spec, old2new) = spec.contiguousify();
        let nodes = spec.generic_nodes().collect::<Vec<_>>();
        let links = spec.generic_links().collect::<Vec<_>>();
        let topology = Topology::new(&nodes, &links)?;
        let routes = Routes::new(&topology);
        Ok(Self {
            topology,
            routes,
            old2new,
        })
    }

    /// Returns `(delay, bandwidth)` pairs from `src` to `dst`, using `choose` to select a path
    /// when there are multiple options.
    pub fn path_info(
        &self,
        src: NodeId,
        dst: NodeId,
        choose: impl FnMut(&[NodeId]) -> Option<&NodeId>,
    ) -> impl Iterator<Item = (Nanosecs, BitsPerSec)> + '_ {
        let src = *self.old2new.get(&src).unwrap();
        let dst = *self.old2new.get(&dst).unwrap();
        self.edge_indices_between(src, dst, choose).map(|eidx| {
            let chan = &self.topology().graph[eidx];
            (chan.delay, chan.bandwidth)
        })
    }
}

impl TraversableNetwork<BasicChannel> for LinkSimTopo {
    fn topology(&self) -> &Topology<BasicChannel> {
        &self.topology
    }

    fn routes(&self) -> &Routes {
        &self.routes
    }
}

/// A descriptor for a link-level simulation.
#[derive(Debug, serde::Serialize)]
pub struct LinkSimDesc {
    /// The bottleneck.
    pub bottleneck: LinkSimLink,
    /// The links other than the bottleneck.
    pub other_links: Vec<LinkSimLink>,
    /// The nodes.
    pub nodes: Vec<LinkSimNode>,
    /// The flow IDs.
    pub flows: Vec<FlowId>,
}

/// A node in a link-level simulation.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct LinkSimNode {
    /// The node ID.
    pub id: NodeId,
    /// The node kind.
    pub kind: LinkSimNodeKind,
}

/// A link in a link-level simulation.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct LinkSimLink {
    /// The first endpoint.
    pub a: NodeId,
    /// The second endpoint.
    pub b: NodeId,
    /// The total link bandwidth.
    pub total_bandwidth: BitsPerSec,
    /// The total bandwidth optionally adjusted for the rate of ACKs
    pub available_bandwidth: BitsPerSec,
    /// The propagation delay.
    pub delay: Nanosecs,
}

/// The types of nodes in a link-level simulation.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum LinkSimNodeKind {
    /// A source node.
    Source,
    /// A destination node.
    Destination,
    /// A switch.
    Switch,
}

/// Link simulation error.
#[derive(Debug, thiserror::Error)]
pub enum LinkSimError {
    /// Tried to simulate a link that doesn't exist.
    #[error("Edge {} does not exist", .0.index())]
    UnknownEdge(EdgeIndex),

    /// IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Topology error.
    #[error(transparent)]
    Topology(#[from] TopologyError),

    /// Arbitrary catch-all.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
