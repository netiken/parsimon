//! This module defines the [`LinkSim`] trait that every link simulator must implement as well as
//! related types.

use std::iter;

use petgraph::prelude::*;
use rustc_hash::FxHashMap;

use crate::{
    network::{
        types::{Link, Node},
        FctRecord, Flow, FlowId, NodeId, NodeKind, TopologyError,
    },
    units::{BitsPerSec, Nanosecs},
};

/// The return type of a link simulation.
pub type LinkSimResult = Result<Vec<FctRecord>, LinkSimError>;

/// An interface for link simulators.
pub trait LinkSim: serde::Serialize + serde::de::DeserializeOwned {
    /// Returns the name of the link level simulator.
    fn name(&self) -> String;

    /// Given [`LinkSimSpec`], simulate it and return a collection of FCT records.
    fn simulate(&self, spec: LinkSimSpec) -> LinkSimResult;
}

/// A full specification for a link-level simulation.
#[derive(Debug)]
pub struct LinkSimSpec {
    /// The edge index of the isolated link.
    pub edge: usize,
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
    /// Returns the nodes in the spec;
    pub fn nodes(&self) -> impl Iterator<Item = LinkSimNode> + '_ {
        self.nodes.iter().copied()
    }

    /// Returns the links in the spec.
    pub fn links(&self) -> impl Iterator<Item = LinkSimLink> + '_ {
        iter::once(&self.bottleneck)
            .chain(self.other_links.iter())
            .copied()
    }

    /// Returns the nodes in the spec, erasing any `LinkSim`-specific information.
    pub fn generic_nodes(&self) -> impl Iterator<Item = Node> + '_ {
        self.nodes().map(|n| {
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
        self.links()
            .map(|l| Link::new(l.from, l.to, l.available_bandwidth, l.delay))
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
                edge: self.edge,
                bottleneck: LinkSimLink {
                    from: *old2new.get(&self.bottleneck.from).unwrap(),
                    to: *old2new.get(&self.bottleneck.to).unwrap(),
                    ..self.bottleneck
                },
                other_links: self
                    .other_links
                    .iter()
                    .map(|&l| LinkSimLink {
                        from: *old2new.get(&l.from).unwrap(),
                        to: *old2new.get(&l.to).unwrap(),
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
}

/// A descriptor for a link-level simulation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LinkSimDesc {
    /// The edge index of the isolated link.
    pub edge: usize,
    /// The bottleneck.
    pub bottleneck: LinkSimLink,
    /// The links other than the bottleneck.
    pub other_links: Vec<LinkSimLink>,
    /// The nodes.
    pub nodes: Vec<LinkSimNode>,
    /// The flow IDs.
    pub flows: Vec<FlowId>,
}

/// A link-level topology.
#[derive(Debug)]
pub struct LinkSimTopo {
    graph: DiGraph<LinkSimNode, LinkSimLink>,
    nid2nix: FxHashMap<NodeId, NodeIndex>,
}

impl LinkSimTopo {
    /// Creates a new link level topology.
    pub fn new(spec: &LinkSimSpec) -> Self {
        let mut graph = DiGraph::new();
        let mut nid2nix = FxHashMap::default();
        for n in spec.nodes() {
            let nix = graph.add_node(n);
            nid2nix.insert(n.id, nix);
        }
        for l in spec.links() {
            let _ = graph.add_edge(
                *nid2nix.get(&l.from).unwrap(),
                *nid2nix.get(&l.to).unwrap(),
                l,
            );
        }
        Self { graph, nid2nix }
    }

    /// Returns a path of `LinkSimLink`s from `src` to `dst.
    pub fn path(&self, src: NodeId, dst: NodeId) -> Option<Vec<LinkSimLink>> {
        let mut path = Vec::new();
        let mut cur = src;
        while cur != dst {
            let nix = *self.nid2nix.get(&cur).unwrap();
            let l = match self
                .graph
                .edges_directed(nix, Direction::Outgoing)
                .find(|l| l.weight().to == dst)
            {
                Some(l) => *l.weight(),
                None => {
                    let eix = match self.graph.first_edge(nix, Direction::Outgoing) {
                        Some(eidx) => eidx,
                        None => return None,
                    };
                    self.graph[eix]
                }
            };
            path.push(l);
            cur = l.to;
        }
        Some(path)
    }
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
    pub from: NodeId,
    /// The second endpoint.
    pub to: NodeId,
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
