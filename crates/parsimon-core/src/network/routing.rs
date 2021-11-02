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

#[derive(Debug, Clone, serde::Serialize)]
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
    use std::collections::BTreeMap;

    use super::*;
    use crate::network::types::{Link, Node};
    use anyhow::Context;

    type SortedHopMatrix = BTreeMap<NodeId, SortedHopMap>;
    type SortedHopMap = BTreeMap<NodeId, Vec<NodeId>>;

    fn sorted_hop_matrix(matrix: &HopMatrix) -> SortedHopMatrix {
        todo!()
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
        insta::assert_yaml_snapshot!(routes, @r###"
        ---
        inner:
          0:
            2:
              - 2
            1:
              - 2
          2:
            1:
              - 1
            0:
              - 0
          1:
            0:
              - 2
            2:
              - 2
        "###);
        Ok(())
    }
}
