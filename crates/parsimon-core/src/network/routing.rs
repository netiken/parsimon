use rustc_hash::FxHashMap;
use std::collections::VecDeque;

use petgraph::{
    graph::NodeIndex,
    visit::{VisitMap, Visitable},
};
use rayon::prelude::*;

use crate::network::{
    topology::Topology,
    types::{BasicChannel, NodeId, NodeKind},
};

// type HopMatrix = FxHashMap<NodeId, HopMap>;
// type HopMap = FxHashMap<NodeId, Vec<NodeId>>;
type HopMatrix = Vec<HopMap>;
type HopMap = Vec<Vec<NodeId>>;

#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct Routes {
    inner: HopMatrix,
}

impl Routes {
    /// Builds a routing table from a topology using BFS.
    pub(super) fn new(topology: &Topology<BasicChannel>) -> Self {
        let g = &topology.graph;

        // Each node is the starting point for a BFS
        let (s, r) = crossbeam_channel::unbounded();
        let nr_cpus = num_cpus::get();
        let node_indices = g.node_indices().collect::<Vec<_>>();
        let nr_nodes = node_indices.len();
        let chunk_size = std::cmp::max(nr_nodes / nr_cpus, 1);
        node_indices
            .chunks(chunk_size)
            .par_bridge()
            .for_each_with(s, |s, indices| {
                let mut hops = Vec::new();
                for &start in indices {
                    let mut discovered = g.visit_map();
                    discovered.visit(start);

                    let mut queue = VecDeque::new();
                    queue.push_back(start);

                    let mut distances: FxHashMap<NodeIndex, usize> =
                        [(start, 0)].into_iter().collect();

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
                                hops.push((g[succ].id, g[start].id, g[n].id))
                            }
                        }
                    }
                }
                s.send(hops).unwrap();
            });

        // Merge the results into a single collection
        let mut hops = vec![vec![Vec::new(); nr_nodes]; nr_nodes];
        for (a, b, c) in r.into_iter().map(|v| v.into_iter()).flatten() {
            hops[a.inner()][b.inner()].push(c);
        }

        Self { inner: hops }
    }

    pub(super) fn for_node(&self, node: NodeId) -> Option<&HopMap> {
        self.inner.get(node.inner())
    }

    pub(super) fn next_hops_unchecked(&self, from: NodeId, to: NodeId) -> &[NodeId] {
        self.for_node(from)
            .expect("missing node in routes")
            .get(to.inner())
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
            .enumerate()
            .map(|(i, m)| {
                let m = m
                    .iter()
                    .enumerate()
                    .map(|(i, hops)| {
                        let mut hops = hops.clone();
                        hops.sort();
                        (NodeId::new(i), hops)
                    })
                    .collect::<BTreeMap<_, _>>();
                (NodeId::new(i), m)
            })
            .collect::<BTreeMap<_, _>>()
    }

    #[test]
    fn route_three_node_works() -> anyhow::Result<()> {
        let (nodes, links) = testing::three_node_config();
        let topo = Topology::new(&nodes, &links).context("failed to create topology")?;
        let routes = Routes::new(&topo);
        let hops = sorted_hop_matrix(&routes.inner);
        insta::assert_yaml_snapshot!(hops);
        Ok(())
    }

    #[test]
    fn route_eight_node_works() -> anyhow::Result<()> {
        let (nodes, links) = testing::eight_node_config();
        let topo = Topology::new(&nodes, &links).context("failed to create topology")?;
        let routes = Routes::new(&topo);
        let hops = sorted_hop_matrix(&routes.inner);
        insta::assert_yaml_snapshot!(hops);
        Ok(())
    }
}
