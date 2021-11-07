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
    use crate::testing;
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
        let topo = testing::three_node_topology().context("failed to create topology")?;
        let routes = Routes::new(&topo);
        let hops = sorted_hop_matrix(&routes.inner);
        insta::assert_yaml_snapshot!(hops);
        Ok(())
    }

    #[test]
    fn route_eight_node_succeeds() -> anyhow::Result<()> {
        let topo = testing::eight_node_topology().context("failed to create topology")?;
        let routes = Routes::new(&topo);
        let hops = sorted_hop_matrix(&routes.inner);
        insta::assert_yaml_snapshot!(hops);
        Ok(())
    }
}
