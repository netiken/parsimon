//! This module defines the manner in which routes are specified and implemented.

use rustc_hash::FxHashMap;
use std::collections::VecDeque;

use petgraph::{
    graph::NodeIndex,
    visit::{VisitMap, Visitable},
};

use crate::{
    network::{
        topology::Topology,
        types::{BasicChannel, NodeId, NodeKind},
    },
    utils,
};

/// The trait implemented by all routing algorithms
pub trait RoutingAlgo {
    /// Return the set of next hops needed to get from `from` to `to.
    fn next_hops(&self, from: NodeId, to: NodeId) -> Option<Vec<NodeId>>;
}

type HopMatrix = Vec<HopMap>;
type HopMap = Vec<Vec<NodeId>>;

/// A routing matrix constructed with BFS.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BfsRoutes {
    inner: HopMatrix,
}

impl BfsRoutes {
    /// Builds a routing table from a topology using BFS.
    pub fn new(topology: &Topology<BasicChannel>) -> Self {
        let g = &topology.graph;

        // Each node is the starting point for a BFS. Do chunks of these in parallel.
        let node_indices = g.node_indices().collect::<Vec<_>>();
        let entries = utils::par_chunks(&node_indices, |indices| {
            let mut entries = Vec::new();
            for &start in indices {
                let mut discovered = g.visit_map();
                discovered.visit(start);

                let mut queue = VecDeque::new();
                queue.push_back(start);

                let mut distances: FxHashMap<NodeIndex, usize> = [(start, 0)].into_iter().collect();

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
                            // You can get from `succ` to `start` through `n`
                            entries.push((g[succ].id, g[start].id, g[n].id))
                        }
                    }
                }
            }
            entries
        });

        // Merge the results into a single collection
        let nr_nodes = node_indices.len();
        let mut hops = vec![vec![Vec::new(); nr_nodes]; nr_nodes];
        for (a, b, c) in entries {
            hops[a.inner()][b.inner()].push(c);
        }

        Self { inner: hops }
    }

    fn for_node(&self, node: NodeId) -> Option<&HopMap> {
        self.inner.get(node.inner())
    }
}

impl RoutingAlgo for BfsRoutes {
    fn next_hops(&self, from: NodeId, to: NodeId) -> Option<Vec<NodeId>> {
        self.for_node(from)
            .and_then(|map| map.get(to.inner()))
            .map(|hops| hops.to_vec())
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
        let routes = BfsRoutes::new(&topo);
        let hops = sorted_hop_matrix(&routes.inner);
        insta::assert_yaml_snapshot!(hops);
        Ok(())
    }

    #[test]
    fn route_eight_node_works() -> anyhow::Result<()> {
        let (nodes, links) = testing::eight_node_config();
        let topo = Topology::new(&nodes, &links).context("failed to create topology")?;
        let routes = BfsRoutes::new(&topo);
        let hops = sorted_hop_matrix(&routes.inner);
        insta::assert_yaml_snapshot!(hops);
        Ok(())
    }
}
