use std::collections::HashSet;

use petgraph::graph::DiGraph;

use crate::network::types::{Channel, Link, Node, NodeId};

#[derive(Debug)]
pub(crate) struct Topology {
    pub(crate) graph: DiGraph<Node, Channel>,
}

impl Topology {
    // Correctness properties:
    //
    // - Every node must have a unique ID.
    // - Every link must have distinct endpoints in `nodes`.
    // - For any two nodes, there must be at most one link between them.
    // - Every host node should only be attached to one link
    pub(crate) fn new(nodes: Vec<Node>, links: Vec<Link>) -> Result<Self, Error> {
        // Every node must have a unique ID.
        let mut node_ids = HashSet::new();
        for Node { id, .. } in &nodes {
            if !node_ids.insert(*id) {
                // ID was already in the set
                return Err(Error::DuplicateNodeId(*id));
            }
        }

        // Every link must have distinct endpoints in `nodes`.
        for Link { a, b, .. } in &links {
            todo!()
        }

        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Duplicate node ID {0}")]
    DuplicateNodeId(NodeId),
}
