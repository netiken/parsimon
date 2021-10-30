use petgraph::graph::DiGraph;

use crate::network::types::{Channel, Link, Node};

#[derive(Debug)]
pub(crate) struct Topology {
    pub(crate) graph: DiGraph<Node, Channel>,
}

impl Topology {
    // Every node should have a unique ID.
    // Every link should have distinct endpoints.
    // For any two nodes, there should only be one link between them.
    // Every node that is a host should only be attached to one link
    pub(crate) fn new(nodes: Vec<Node>, links: Vec<Link>) -> Self {
        todo!()
    }
}
