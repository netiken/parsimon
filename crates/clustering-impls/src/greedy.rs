use std::collections::{HashSet, LinkedList};

use parsimon_core::{
    cluster::{Cluster, ClusteringAlgo},
    network::{Flow, SimNetwork},
};

#[derive(Debug, derive_new::new)]
pub struct GreedyClustering<R, D> {
    epsilon: R,
    distance: D,
}

impl<R, D> ClusteringAlgo for GreedyClustering<R, D>
where
    R: Clone + Copy + PartialOrd,
    D: Fn(&[Flow], &[Flow]) -> R,
{
    fn cluster(&self, network: &SimNetwork) -> Vec<Cluster> {
        let mut unclustered_edges = network.edge_indices().collect::<LinkedList<_>>();
        let mut clusters = Vec::new();
        // Every time we remove an element, it becomes the next cluster representative.
        let mut cursor = unclustered_edges.cursor_front_mut();
        while let Some(representative) = cursor.remove_current() {
            let rflows = network.flows_on(representative).unwrap();
            let mut members = [representative].into_iter().collect::<HashSet<_>>();
            // Check all other unclustered edges to see if they're within epsilon of the current
            // representative.
            while let Some(&mut candidate) = cursor.current() {
                let cflows = network.flows_on(candidate).unwrap();
                if (self.distance)(&rflows, &cflows) <= self.epsilon {
                    members.insert(candidate);
                    cursor.remove_current();
                } else {
                    cursor.move_next();
                }
            }
            // Because of the above iteration, the cursor now points to the ghost element, so
            // circle it back around.
            cursor.move_next();
            // We're done with this cluster.
            clusters.push(Cluster::new(representative, members));
        }
        clusters
    }
}
