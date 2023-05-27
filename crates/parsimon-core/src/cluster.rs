//! This module defines types and traits which allow link clustering and pruning.

use std::collections::HashSet;

use petgraph::graph::EdgeIndex;

use crate::{network::SimNetwork, routing::RoutingAlgo};

/// A cluster of edges with a representative member.
#[derive(Debug, Clone, derive_new::new)]
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

/// The trait that must be implemented by all clustering algorithms.
pub trait ClusteringAlgo {
    /// Given a [`SimNetwork`], run a clustering algorithm and return a vector of
    /// [clusters](Cluster).
    fn cluster<R>(&self, network: &SimNetwork<R>) -> Vec<Cluster>
    where
        R: RoutingAlgo + Sync;
}

impl<C: ClusteringAlgo> ClusteringAlgo for &C {
    fn cluster<R>(&self, network: &SimNetwork<R>) -> Vec<Cluster>
    where
        R: RoutingAlgo + Sync,
    {
        (*self).cluster(network)
    }
}

/// A no-op clustering algorithm.
#[derive(Debug)]
pub struct DefaultClustering;

impl ClusteringAlgo for DefaultClustering {
    fn cluster<R>(&self, network: &SimNetwork<R>) -> Vec<Cluster>
    where
        R: RoutingAlgo + Sync,
    {
        network.clusters().to_vec()
    }
}
