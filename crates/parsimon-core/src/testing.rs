use crate::network::types::{Link, Node, NodeId};

pub(crate) fn three_node_config() -> (Vec<Node>, Vec<Link>) {
    let n1 = Node::new_host(NodeId::new(0));
    let n2 = Node::new_host(NodeId::new(1));
    let n3 = Node::new_switch(NodeId::new(2));
    let l1 = Link::new(n1.id, n3.id);
    let l2 = Link::new(n2.id, n3.id);
    (vec![n1, n2, n3], vec![l1, l2])
}

pub(crate) fn eight_node_config() -> (Vec<Node>, Vec<Link>) {
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
    (nodes, links)
}
