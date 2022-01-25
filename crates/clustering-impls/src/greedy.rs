use std::collections::HashSet;

use dashmap::DashMap;
use parsimon_core::{
    cluster::{Cluster, ClusteringAlgo},
    network::{EdgeIndex, Flow, SimNetwork},
};
use rayon::prelude::*;
use rustc_hash::FxHashSet;

#[derive(Debug, derive_new::new)]
pub struct GreedyClustering<E, F, D> {
    epsilon: E,
    feature: F,
    distance: D,
}

impl<E, F, D, X> ClusteringAlgo for GreedyClustering<E, F, D>
where
    E: Clone + Copy + PartialOrd + Sync,
    F: Fn(&[Flow]) -> X + Sync,
    D: Fn(&X, &X) -> E + Sync,
    X: Clone + Send + Sync,
{
    fn cluster(&self, network: &SimNetwork) -> Vec<Cluster> {
        let features = Features::new(network, &self.feature);
        let mut unclustered_edges = network.edge_indices().collect::<FxHashSet<_>>();
        let mut clusters = Vec::new();
        // Every time we remove an element, it becomes the next cluster representative.
        while let Some(&representative) = unclustered_edges.iter().next() {
            unclustered_edges.remove(&representative);
            let rfeat = features.get(representative);
            let mut members = [representative].into_iter().collect::<HashSet<_>>();
            // Check all other unclustered edges to see if they're within epsilon of the current
            // representative.
            let candidates = unclustered_edges
                .par_iter()
                .filter_map(|&candidate| {
                    let cfeat = features.get(candidate);
                    ((self.distance)(&rfeat, &cfeat) <= self.epsilon).then(|| candidate)
                })
                .collect::<Vec<_>>();
            for candidate in candidates {
                members.insert(candidate);
                unclustered_edges.remove(&candidate);
            }
            // We're done with this cluster.
            // println!("Cluster {} has {} members", clusters.len(), members.len());
            clusters.push(Cluster::new(representative, members));
        }
        clusters
    }
}

#[derive(derive_new::new)]
struct Features<'a, F, X> {
    network: &'a SimNetwork,
    feature: F,
    #[new(default)]
    cache: DashMap<EdgeIndex, X>,
}

impl<'a, F, X> Features<'a, F, X>
where
    F: Fn(&[Flow]) -> X + Sync,
    X: Clone + Send + Sync,
{
    fn get(&self, eidx: EdgeIndex) -> X {
        self.cache
            .entry(eidx)
            .or_insert_with(|| {
                let flows = self
                    .network
                    .flows_on(eidx)
                    .expect("invalid `eidx` in `Features::get`");
                (self.feature)(&flows)
            })
            .clone()
    }
}
