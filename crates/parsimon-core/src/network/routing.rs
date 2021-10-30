use std::collections::{HashMap, VecDeque};

use petgraph::{
    graph::NodeIndex,
    visit::{VisitMap, Visitable},
};

use crate::network::{
    topology::Topology,
    types::{NodeId, NodeKind},
};

pub(super) type HopMatrix = HashMap<NodeId, HopMap>;
pub(super) type HopMap = HashMap<NodeId, Vec<NodeId>>;

#[derive(Debug, Clone)]
pub(super) struct Routes {
    inner: HopMatrix,
}

impl Routes {
    /// Builds a routing table from a topology using BFS.
    pub(super) fn new(topology: &Topology) -> Self {
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
                    // In this function, we do not assume there is a 1:1 mapping between `NodeId`s
                    // and `NodeIndex`s, but it may be enforced elsewhere
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

// TODO: write tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tbd() {
        todo!()
    }
}
