use std::collections::{HashMap, HashSet};

use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};

use crate::network::types::{Channel, Link, Node, NodeId, NodeKind, TracedChannel};

use super::types::EDistChannel;

#[derive(Debug)]
pub(crate) struct Topology<C> {
    pub(crate) graph: DiGraph<Node, C>,
    pub(crate) id2idx: HashMap<NodeId, NodeIndex>,
    pub(crate) links: Vec<Link>,
}

impl<C> Topology<C> {
    delegate::delegate! {
        to self.id2idx {
            #[call(get)]
            pub(crate) fn idx_of(&self, id: &NodeId) -> Option<&NodeIndex>;
        }

        to self.graph {
            #[allow(unused)]
            #[call(edge_count)]
            pub(crate) fn nr_edges(&self) -> usize;

            pub(crate) fn find_edge(&self, a: NodeIndex, b: NodeIndex) -> Option<EdgeIndex>;
        }
    }
}

impl Topology<Channel> {
    /// Creates a network topology from a list of nodes and links. This function returns an error if
    /// the given specification fails to produce a valid topology. The checks are not exhaustive.
    ///
    /// Correctness properties:
    ///
    /// - Every node must have a unique ID.
    /// - Every link must have distinct endpoints in `nodes`.
    /// - Every node must be referenced by some link.
    /// - For any two nodes, there must be at most one link between them.
    /// - Every host node should only have one link.
    pub(crate) fn new(nodes: &[Node], links: &[Link]) -> Result<Self, TopologyError> {
        let mut g = DiGraph::new();
        let mut id2idx = HashMap::new();
        for n @ Node { id, .. } in nodes.iter().cloned() {
            let idx = g.add_node(n);
            if id2idx.insert(id, idx).is_some() {
                // CORRECTNESS: Every node must have a unique ID.
                return Err(TopologyError::DuplicateNodeId(id));
            }
        }
        let idx_of = |id| *id2idx.get(&id).unwrap();
        let mut referenced_nodes = HashSet::new();
        for Link {
            a,
            b,
            bandwidth,
            delay,
        } in links.iter().cloned()
        {
            // CORRECTNESS: Every link must have distinct endpoints in `nodes`.
            if a == b {
                return Err(TopologyError::NodeAdjacentSelf(a));
            }
            if !id2idx.contains_key(&a) {
                return Err(TopologyError::UndeclaredNode(a));
            }
            if !id2idx.contains_key(&b) {
                return Err(TopologyError::UndeclaredNode(b));
            }
            referenced_nodes.insert(a);
            referenced_nodes.insert(b);
            // Channels are unidirectional
            g.add_edge(idx_of(a), idx_of(b), Channel::new(a, b, bandwidth, delay));
            g.add_edge(idx_of(b), idx_of(a), Channel::new(b, a, bandwidth, delay));
        }
        // CORRECTNESS: Every node must be referenced by some link.
        for &id in id2idx.keys() {
            if !referenced_nodes.contains(&id) {
                return Err(TopologyError::IsolatedNode(id));
            }
        }
        for eidx in g.edge_indices() {
            // CORRECTNESS: For any two nodes, there must be at most one link between them.
            let (a, b) = g.edge_endpoints(eidx).unwrap();
            if g.edges_connecting(a, b).count() > 1 {
                return Err(TopologyError::DuplicateLink {
                    n1: g[a].id,
                    n2: g[b].id,
                });
            }
            // CORRECTNESS: Every host node should only have one link.
            let Node { id, kind, .. } = g[a];
            if matches!(kind, NodeKind::Host) {
                let nr_outgoing = g.edges(a).count();
                if nr_outgoing > 1 {
                    return Err(TopologyError::TooManyHostLinks { id, n: nr_outgoing });
                }
            }
        }
        Ok(Self {
            graph: g,
            id2idx,
            links: Vec::from(links),
        })
    }
}

impl Topology<TracedChannel> {
    pub(crate) fn new_traced(topology: &Topology<Channel>) -> Self {
        // CORRECTNESS: For nodes and edges, `petgraph` guarantees that the
        // iteration order matches the order of indices.
        let mut g = DiGraph::new();
        for node in topology.graph.node_weights() {
            g.add_node(node.clone());
        }
        for eidx in topology.graph.edge_indices() {
            let (a, b) = topology.graph.edge_endpoints(eidx).unwrap();
            let chan = &topology.graph[eidx];
            g.add_edge(a, b, TracedChannel::new_from(&chan));
        }
        Topology {
            graph: g,
            id2idx: topology.id2idx.clone(),
            links: topology.links.clone(),
        }
    }
}

impl Topology<EDistChannel> {
    pub(crate) fn new_edist(topology: &Topology<TracedChannel>) -> Self {
        // CORRECTNESS: For nodes and edges, `petgraph` guarantees that the
        // iteration order matches the order of indices.
        let mut g = DiGraph::new();
        for node in topology.graph.node_weights() {
            g.add_node(node.clone());
        }
        for eidx in topology.graph.edge_indices() {
            let (a, b) = topology.graph.edge_endpoints(eidx).unwrap();
            let chan = &topology.graph[eidx];
            g.add_edge(a, b, EDistChannel::new_from(&chan));
        }
        Topology {
            graph: g,
            id2idx: topology.id2idx.clone(),
            links: topology.links.clone(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TopologyError {
    #[error("Duplicate node ID {0}")]
    DuplicateNodeId(NodeId),

    #[error("Node {0} is connected to itself")]
    NodeAdjacentSelf(NodeId),

    #[error("Node {0} is not declared")]
    UndeclaredNode(NodeId),

    #[error("Duplicate links between {n1} and {n2}")]
    DuplicateLink { n1: NodeId, n2: NodeId },

    #[error("Host {id} has too many links (expected 1, got {n})")]
    TooManyHostLinks { id: NodeId, n: usize },

    #[error("Node {0} is not connected to any other node")]
    IsolatedNode(NodeId),
}

#[cfg(test)]
mod tests {
    use anyhow::Context;

    use super::*;
    use crate::testing;
    use crate::units::{Gbps, Nanosecs};

    #[test]
    fn empty_topology_succeeds() {
        assert!(
            Topology::<Channel>::new(&[], &[]).is_ok(),
            "failed to create empty topology"
        );
    }

    #[test]
    fn three_node_topology_works() -> anyhow::Result<()> {
        let (nodes, links) = testing::three_node_config();
        let topo = Topology::new(&nodes, &links).context("failed to create topology")?;
        insta::assert_yaml_snapshot!(topo.graph);
        Ok(())
    }

    #[test]
    fn eight_node_topology_works() -> anyhow::Result<()> {
        let (nodes, links) = testing::eight_node_config();
        let topo = Topology::<Channel>::new(&nodes, &links).context("failed to create topology")?;
        insta::assert_yaml_snapshot!(topo.graph);
        Ok(())
    }

    #[test]
    fn duplicate_node_fails() {
        let n1 = Node::new_host(NodeId::new(0));
        let n2 = Node::new_host(NodeId::new(0)); // error
        let n3 = Node::new_switch(NodeId::new(2));
        let l1 = Link::new(n1.id, n3.id, Gbps::default(), Nanosecs::default());
        let l2 = Link::new(n2.id, n3.id, Gbps::default(), Nanosecs::default());
        let res = Topology::new(&[n1, n2, n3], &[l1, l2]);
        assert!(matches!(res, Err(TopologyError::DuplicateNodeId(..))));
    }

    #[test]
    fn node_adjacent_self_fails() {
        let n1 = Node::new_host(NodeId::new(0));
        let n2 = Node::new_host(NodeId::new(1));
        let n3 = Node::new_switch(NodeId::new(2));
        let l1 = Link::new(n1.id, n3.id, Gbps::default(), Nanosecs::default());
        let l2 = Link::new(n2.id, n3.id, Gbps::default(), Nanosecs::default());
        let l3 = Link::new(n3.id, n3.id, Gbps::default(), Nanosecs::default()); // error
        let res = Topology::new(&[n1, n2, n3], &[l1, l2, l3]);
        assert!(matches!(res, Err(TopologyError::NodeAdjacentSelf(..))));
    }

    #[test]
    fn undeclared_node_fails() {
        let n1 = Node::new_host(NodeId::new(0));
        let n2 = Node::new_host(NodeId::new(1));
        let n3 = Node::new_switch(NodeId::new(2));
        let l1 = Link::new(n1.id, n3.id, Gbps::default(), Nanosecs::default());
        let l2 = Link::new(n2.id, n3.id, Gbps::default(), Nanosecs::default());
        let l3 = Link::new(NodeId::new(3), n3.id, Gbps::default(), Nanosecs::default());
        let res = Topology::new(&[n1, n2, n3], &[l1, l2, l3]);
        assert!(matches!(res, Err(TopologyError::UndeclaredNode(..))));
    }

    #[test]
    fn duplicate_links_fails() {
        let n1 = Node::new_host(NodeId::new(0));
        let n2 = Node::new_host(NodeId::new(1));
        let n3 = Node::new_switch(NodeId::new(2));
        let l1 = Link::new(n1.id, n3.id, Gbps::default(), Nanosecs::default());
        let l2 = Link::new(n2.id, n3.id, Gbps::default(), Nanosecs::default());
        let l3 = Link::new(n2.id, n3.id, Gbps::default(), Nanosecs::default()); // error
        let res = Topology::<Channel>::new(&[n1, n2, n3], &[l1, l2, l3]);
        assert!(matches!(res, Err(TopologyError::DuplicateLink { .. })));
    }

    #[test]
    fn too_many_host_links_fails() {
        let n1 = Node::new_host(NodeId::new(0));
        let n2 = Node::new_host(NodeId::new(1));
        let n3 = Node::new_switch(NodeId::new(2));
        let n4 = Node::new_switch(NodeId::new(3));
        let l1 = Link::new(n1.id, n3.id, Gbps::default(), Nanosecs::default());
        let l2 = Link::new(n2.id, n3.id, Gbps::default(), Nanosecs::default());
        let l3 = Link::new(n1.id, n4.id, Gbps::default(), Nanosecs::default()); // error
        let res = Topology::new(&[n1, n2, n3, n4], &[l1, l2, l3]);
        assert!(matches!(
            res,
            Err(TopologyError::TooManyHostLinks { n: 2, .. })
        ));
    }

    #[test]
    fn isolated_node_fails() {
        let n1 = Node::new_host(NodeId::new(0));
        let n2 = Node::new_host(NodeId::new(1));
        let n3 = Node::new_switch(NodeId::new(2));
        let n4 = Node::new_host(NodeId::new(3)); // error
        let l1 = Link::new(n1.id, n3.id, Gbps::default(), Nanosecs::default());
        let l2 = Link::new(n2.id, n3.id, Gbps::default(), Nanosecs::default());
        let res = Topology::new(&[n1, n2, n3, n4], &[l1, l2]);
        assert!(matches!(res, Err(TopologyError::IsolatedNode(..))));
    }

    #[test]
    fn new_topo_old_topo_equiv() -> anyhow::Result<()> {
        let (nodes, links) = testing::eight_node_config();
        let topo1 =
            Topology::<Channel>::new(&nodes, &links).context("failed to create topology")?;
        let topo2 = Topology::new_traced(&topo1);
        // Iteration order matches the order of indices
        for (n1, n2) in topo1.graph.node_weights().zip(topo2.graph.node_weights()) {
            assert_eq!(n1, n2);
        }
        for (e1, e2) in topo1.graph.edge_weights().zip(topo2.graph.edge_weights()) {
            let e2 = &Channel::new(e2.src, e2.dst, Gbps::new(100), Nanosecs::new(1000));
            assert_eq!(e1, e2);
        }
        Ok(())
    }
}
