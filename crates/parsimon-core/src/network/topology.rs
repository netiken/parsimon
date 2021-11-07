use std::collections::{HashMap, HashSet};

use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};

use crate::{
    network::types::{Channel, Link, Node, NodeId, TracedChannel},
    NodeKind,
};

#[derive(Debug)]
pub(crate) struct Topology<C> {
    pub(crate) graph: DiGraph<Node, C>,
    pub(crate) id2idx: HashMap<NodeId, NodeIndex>,
}

impl<C> Topology<C> {
    delegate::delegate! {
        to self.graph {
            pub(crate) fn find_edge(&self, src: NodeIndex, dst:NodeIndex) -> Option<EdgeIndex>;
        }
        to self.id2idx {
            #[call(get)]
            pub(crate) fn idx_of(&self, id: &NodeId) -> Option<&NodeIndex>;
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
        for Link { a, b, .. } in links.iter().cloned() {
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
            g.add_edge(idx_of(a), idx_of(b), Channel::new(a, b));
            g.add_edge(idx_of(b), idx_of(a), Channel::new(b, a));
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
        Ok(Self { graph: g, id2idx })
    }
}

impl Topology<TracedChannel> {
    pub(crate) fn new_empty(topology: &Topology<Channel>) -> Self {
        // CORRECTNESS: For nodes and edges, `petgraph` guarantees that the
        // iteration order matches the order of indices.
        let mut g = DiGraph::new();
        for node in topology.graph.node_weights() {
            g.add_node(node.clone());
        }
        for eidx in topology.graph.edge_indices() {
            let (a, b) = topology.graph.edge_endpoints(eidx).unwrap();
            let chan = &topology.graph[eidx];
            g.add_edge(a, b, TracedChannel::new_empty(&chan));
        }
        Topology {
            graph: g,
            id2idx: topology.id2idx.clone(),
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

    #[error("Host {id} has too many links (expected 1, got {n}")]
    TooManyHostLinks { id: NodeId, n: usize },

    #[error("Node {0} is not connected to any other node")]
    IsolatedNode(NodeId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_topology_succeeds() {
        assert!(
            Topology::new(&[], &[]).is_ok(),
            "failed to create empty topology"
        );
    }

    #[test]
    fn three_node_topology_succeeds() {
        let n1 = Node::new_host(NodeId::new(0));
        let n2 = Node::new_host(NodeId::new(1));
        let n3 = Node::new_switch(NodeId::new(2));
        let l1 = Link::new(n1.id, n3.id);
        let l2 = Link::new(n2.id, n3.id);
        let res = Topology::new(&[n1, n2, n3], &[l1, l2]);
        assert!(res.is_ok());
    }

    #[test]
    fn duplicate_node_fails() {
        let n1 = Node::new_host(NodeId::new(0));
        let n2 = Node::new_host(NodeId::new(0)); // error
        let n3 = Node::new_switch(NodeId::new(2));
        let l1 = Link::new(n1.id, n3.id);
        let l2 = Link::new(n2.id, n3.id);
        let res = Topology::new(&[n1, n2, n3], &[l1, l2]);
        assert!(matches!(res, Err(TopologyError::DuplicateNodeId(..))));
    }

    #[test]
    fn node_adjacent_self_fails() {
        let n1 = Node::new_host(NodeId::new(0));
        let n2 = Node::new_host(NodeId::new(1));
        let n3 = Node::new_switch(NodeId::new(2));
        let l1 = Link::new(n1.id, n3.id);
        let l2 = Link::new(n2.id, n3.id);
        let l3 = Link::new(n3.id, n3.id); // error
        let res = Topology::new(&[n1, n2, n3], &[l1, l2, l3]);
        assert!(matches!(res, Err(TopologyError::NodeAdjacentSelf(..))));
    }

    #[test]
    fn undeclared_node_fails() {
        let n1 = Node::new_host(NodeId::new(0));
        let n2 = Node::new_host(NodeId::new(1));
        let n3 = Node::new_switch(NodeId::new(2));
        let l1 = Link::new(n1.id, n3.id);
        let l2 = Link::new(n2.id, n3.id);
        let l3 = Link::new(NodeId::new(3), n3.id);
        let res = Topology::new(&[n1, n2, n3], &[l1, l2, l3]);
        assert!(matches!(res, Err(TopologyError::UndeclaredNode(..))));
    }

    #[test]
    fn duplicate_links_fails() {
        let n1 = Node::new_host(NodeId::new(0));
        let n2 = Node::new_host(NodeId::new(1));
        let n3 = Node::new_switch(NodeId::new(2));
        let l1 = Link::new(n1.id, n3.id);
        let l2 = Link::new(n2.id, n3.id);
        let l3 = Link::new(n2.id, n3.id); // error
        let res = Topology::new(&[n1, n2, n3], &[l1, l2, l3]);
        assert!(matches!(res, Err(TopologyError::DuplicateLink { .. })));
    }

    #[test]
    fn too_many_host_links_fails() {
        let n1 = Node::new_host(NodeId::new(0));
        let n2 = Node::new_host(NodeId::new(1));
        let n3 = Node::new_switch(NodeId::new(2));
        let n4 = Node::new_switch(NodeId::new(3));
        let l1 = Link::new(n1.id, n3.id);
        let l2 = Link::new(n2.id, n3.id);
        let l3 = Link::new(n1.id, n4.id); // error
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
        let l1 = Link::new(n1.id, n3.id);
        let l2 = Link::new(n2.id, n3.id);
        let res = Topology::new(&[n1, n2, n3, n4], &[l1, l2]);
        assert!(matches!(res, Err(TopologyError::IsolatedNode(..))));
    }

    #[test]
    fn eight_node_topology_succeeds() {
        // 4 hosts (IDs 0-3), 4 switches (IDs 4 and 5 are ToRs, IDs 6 and 7 are Aggs)
        let hosts = (0..=3).map(|i| Node::new_host(NodeId::new(i)));
        let switches = (4..=7).map(|i| Node::new_switch(NodeId::new(i)));
        let nodes = hosts.chain(switches).collect::<Vec<_>>();
        // Each ToR is connected to 2 hosts
        let mut links = Vec::new();
        links.push(Link::new(nodes[0].id, nodes[4].id));
        links.push(Link::new(nodes[1].id, nodes[4].id));
        links.push(Link::new(nodes[2].id, nodes[5].id));
        links.push(Link::new(nodes[3].id, nodes[5].id));
        // Each ToR is connected to both Aggs
        links.push(Link::new(nodes[4].id, nodes[6].id));
        links.push(Link::new(nodes[4].id, nodes[7].id));
        links.push(Link::new(nodes[5].id, nodes[6].id));
        links.push(Link::new(nodes[5].id, nodes[7].id));
        let res = Topology::new(&nodes, &links);
        assert!(res.is_ok());
    }
}
