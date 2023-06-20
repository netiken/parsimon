//! A greedy link clustering algorithm.

use std::collections::HashSet;

use indicatif::ProgressBar;
use log::info;

use dashmap::DashMap;
use parsimon_core::{
    cluster::{Cluster, ClusteringAlgo},
    network::{types::FlowChannel, EdgeIndex, Flow, SimNetwork},
    routing::RoutingAlgo,
};
use rayon::prelude::*;
use rustc_hash::FxHashSet;

/// Greedy clustering. This algorithm arbitrarily selects a link and clusters it with all links
/// that are "close" to it. Then, it repeats the process with another arbitrary unclustered link,
/// and so on.
#[derive(Debug, derive_new::new)]
pub struct GreedyClustering<F, G> {
    feature: F,
    is_close_enough: G,
}

impl<F, G, X> ClusteringAlgo for GreedyClustering<F, G>
where
    F: Fn(&FlowChannel, &[Flow]) -> X + Sync,
    G: Fn(&X, &X) -> bool + Sync,
    X: Clone + Send + Sync,
{
    fn cluster<R>(&self, network: &SimNetwork<R>) -> Vec<Cluster>
    where
        R: RoutingAlgo + Sync,
    {
        let features = Features::new(network, &self.feature);
        let mut unclustered_edges = network.edge_indices().collect::<FxHashSet<_>>();
        let mut clusters = Vec::new();
        info!("Clustering edges into representatives");
        let bar = ProgressBar::new_spinner();
        // Every time we remove an element, it becomes the next cluster representative.
        while let Some(&representative) = unclustered_edges.iter().next() {
            let num_clusters_total = clusters.len();
            let num_edges_left = unclustered_edges.len();
            bar.set_message(format!(
                "Processing {num_clusters_total} representative. {num_edges_left} unclustered edges remaining"
            ));
            unclustered_edges.remove(&representative);
            let rfeat = features.get(representative);
            let mut members = [representative].into_iter().collect::<HashSet<_>>();
            // Check all other unclustered edges to see if they're within epsilon of the current
            // representative.
            let candidates = unclustered_edges
                .par_iter()
                .filter_map(|&candidate| {
                    let cfeat = features.get(candidate);
                    (self.is_close_enough)(&rfeat, &cfeat).then_some(candidate)
                })
                .collect::<Vec<_>>();
            for candidate in candidates {
                members.insert(candidate);
                unclustered_edges.remove(&candidate);
            }
            // We're done with this cluster.
            clusters.push(Cluster::new(representative, members));
        }
        bar.finish_and_clear();
        clusters
    }
}

#[derive(derive_new::new)]
struct Features<'a, F, X, R> {
    network: &'a SimNetwork<R>,
    feature: F,
    #[new(default)]
    cache: DashMap<EdgeIndex, X>,
}

impl<'a, F, X, R> Features<'a, F, X, R>
where
    F: Fn(&FlowChannel, &[Flow]) -> X + Sync,
    X: Clone + Send + Sync,
    R: RoutingAlgo + Sync,
{
    fn get(&self, eidx: EdgeIndex) -> X {
        self.cache
            .entry(eidx)
            .or_insert_with(|| {
                let chan = self
                    .network
                    .edge(eidx)
                    .expect("invalid `eidx` in `Features::get`");
                let flows = self
                    .network
                    .flows_on(eidx)
                    .expect("invalid `eidx` in `Features::get`");
                (self.feature)(chan, &flows)
            })
            .clone()
    }
}
