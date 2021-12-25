use std::collections::HashSet;

use petgraph::graph::EdgeIndex;

use crate::network::SimNetwork;

/// A cluster of edges with a representative member.
#[derive(Debug, derive_new::new)]
pub struct Cluster {
    representative: EdgeIndex,
    members: HashSet<EdgeIndex>,
}

impl Cluster {
    /// Get a reference to the cluster's representative.
    pub fn representative(&self) -> EdgeIndex {
        self.representative
    }

    delegate::delegate! {
        to self.members {
            /// Returns true if the cluster contains the edge `eidx`.
            pub fn contains(&self, eidx: &EdgeIndex) -> bool;

            /// Returns an iterator over the edge indices of the cluster's members.
            #[call(iter)]
            pub fn members(&self) -> impl Iterator<Item = &EdgeIndex>;
        }
    }
}

pub trait ClusteringAlgo {
    fn cluster(&self, network: &mut SimNetwork);
}

impl<C: ClusteringAlgo> ClusteringAlgo for &C {
    fn cluster(&self, network: &mut SimNetwork) {
        network.cluster(*self);
    }
}

#[derive(Debug)]
pub struct DefaultClustering;

impl ClusteringAlgo for DefaultClustering {
    fn cluster(&self, _: &mut SimNetwork) {}
}
