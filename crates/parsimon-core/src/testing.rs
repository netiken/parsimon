//! Utilities for writing tests.

use crate::network::types::{Link, Node, NodeId};
use crate::units::{Gbps, Nanosecs};

/// Generate a configuration with two hosts connected by a switch.
///
/// Links are 10 Gbps with a 1 us propagation delay.
pub fn three_node_config() -> (Vec<Node>, Vec<Link>) {
    let n1 = Node::new_host(NodeId::new(0));
    let n2 = Node::new_host(NodeId::new(1));
    let n3 = Node::new_switch(NodeId::new(2));
    let l1 = Link::new(n1.id, n3.id, Gbps::new(10), Nanosecs::new(1000));
    let l2 = Link::new(n2.id, n3.id, Gbps::new(10), Nanosecs::new(1000));
    (vec![n1, n2, n3], vec![l1, l2])
}

/// Generate a configuration with four hosts (IDs 0-3), two ToR switches (IDs 4-5), and two agg
/// switches (IDs 6-7) organized in a Clos topology. Each ToR is connected to two hosts.
///
/// Links are 10 Gbps with a 1 us propagation delay.
pub fn eight_node_config() -> (Vec<Node>, Vec<Link>) {
    let hosts = (0..=3).map(|i| Node::new_host(NodeId::new(i)));
    let switches = (4..=7).map(|i| Node::new_switch(NodeId::new(i)));
    let nodes = hosts.chain(switches).collect::<Vec<_>>();
    let links = vec![
        Link::new(nodes[0].id, nodes[4].id, Gbps::new(10), Nanosecs::new(1000)),
        Link::new(nodes[1].id, nodes[4].id, Gbps::new(10), Nanosecs::new(1000)),
        Link::new(nodes[2].id, nodes[5].id, Gbps::new(10), Nanosecs::new(1000)),
        Link::new(nodes[3].id, nodes[5].id, Gbps::new(10), Nanosecs::new(1000)),
        // Each ToR is connected to both Aggs
        Link::new(nodes[4].id, nodes[6].id, Gbps::new(10), Nanosecs::new(1000)),
        Link::new(nodes[4].id, nodes[7].id, Gbps::new(10), Nanosecs::new(1000)),
        Link::new(nodes[5].id, nodes[6].id, Gbps::new(10), Nanosecs::new(1000)),
        Link::new(nodes[5].id, nodes[7].id, Gbps::new(10), Nanosecs::new(1000)),
    ];
    (nodes, links)
}
