use std::collections::{HashMap, VecDeque};

use petgraph::{
    graph::NodeIndex,
    visit::{VisitMap, Visitable},
};

use crate::network::{
    topology::Topology,
    types::{Channel, NodeId, NodeKind},
};

pub(super) type HopMatrix = HashMap<NodeId, HopMap>;
pub(super) type HopMap = HashMap<NodeId, Vec<NodeId>>;

#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct Routes {
    inner: HopMatrix,
}

impl Routes {
    /// Builds a routing table from a topology using BFS.
    pub(super) fn new(topology: &Topology<Channel>) -> Self {
        let g = &topology.graph;
        let mut hops = HopMatrix::new();
        for start in g.node_indices() {
            let mut discovered = g.visit_map();
            discovered.visit(start);

            let mut queue = VecDeque::new();
            queue.push_back(start);

            let mut distances: HashMap<NodeIndex, usize> = [(start, 0)].into_iter().collect();

            while let Some(n) = queue.pop_front() {
                let cur_distance = *distances.get(&n).unwrap();
                for succ in g.neighbors(n) {
                    if discovered.visit(succ) {
                        distances.insert(succ, cur_distance + 1);
                        if matches!(g[succ].kind, NodeKind::Switch) {
                            queue.push_back(succ);
                        }
                    }
                    // In this function, we do not assume `NodeId`s and `NodeIndex`s are exactly
                    // the same, but it may be enforced elsewhere
                    if *distances.get(&succ).unwrap() == cur_distance + 1 {
                        hops.entry(g[succ].id)
                            .or_default()
                            .entry(g[start].id)
                            .or_default()
                            .push(g[n].id);
                    }
                }
            }
        }
        Self { inner: hops }
    }

    pub(super) fn for_node(&self, node: NodeId) -> Option<&HopMap> {
        self.inner.get(&node)
    }

    pub(super) fn next_hops_unchecked(&self, from: NodeId, to: NodeId) -> &[NodeId] {
        self.for_node(from)
            .expect("missing node in routes")
            .get(&to)
            .expect("missing route to node")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::network::types::{Link, Node};
    use anyhow::Context;

    type SortedHopMatrix = BTreeMap<NodeId, SortedHopMap>;
    type SortedHopMap = BTreeMap<NodeId, Vec<NodeId>>;

    /// Generate a stable sorting of the hop matrix for snapshot tests
    fn sorted_hop_matrix(matrix: &HopMatrix) -> SortedHopMatrix {
        matrix
            .iter()
            .map(|(&id, m)| {
                let m = m
                    .iter()
                    .map(|(&id, hops)| {
                        let mut hops = hops.clone();
                        hops.sort();
                        (id, hops)
                    })
                    .collect::<BTreeMap<_, _>>();
                (id, m)
            })
            .collect::<BTreeMap<_, _>>()
    }

    #[test]
    fn route_three_node_succeeds() -> anyhow::Result<()> {
        let n1 = Node::new_host(NodeId::new(0));
        let n2 = Node::new_host(NodeId::new(1));
        let n3 = Node::new_switch(NodeId::new(2));
        let l1 = Link::new(n1.id, n3.id);
        let l2 = Link::new(n2.id, n3.id);
        let topo = Topology::new(&[n1, n2, n3], &[l1, l2]).context("failed to create topology")?;
        let routes = Routes::new(&topo);
        let hops = sorted_hop_matrix(&routes.inner);
        insta::assert_yaml_snapshot!(hops);
        Ok(())
    }

    #[test]
    fn route_eight_node_succeeds() -> anyhow::Result<()> {
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
        let topo = Topology::new(&nodes, &links).context("failed to create topology")?;
        let routes = Routes::new(&topo);
        let hops = sorted_hop_matrix(&routes.inner);
        insta::assert_yaml_snapshot!(hops);
        Ok(())
    }
}
