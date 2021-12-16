use crate::units::Nanosecs;

/// This trait defines an interface for aggregating link-level delay predictions into path
/// predictions.
// XXX: These independence corrections make dynamic convolution difficult
pub trait Aggregator {
    /// PRECONDITION: adjacent links have adjacent entries in `predictions`
    fn aggregate(&self, predictions: &[LinkPrediction]) -> Nanosecs;
}

impl<T: Aggregator> Aggregator for &T {
    fn aggregate(&self, predictions: &[LinkPrediction]) -> Nanosecs {
        (**self).aggregate(predictions)
    }
}

// XXX: Should this be bandwidth headroom instead of offered loads? What correlates the most with
// delay as we've defined it?
#[derive(Debug)]
pub struct LinkPrediction<'a> {
    pub delay: Nanosecs,
    pub offered_loads: &'a [f64],
}

/// This aggregator sums link predictions without making any corrections based on their offered
/// loads.
#[derive(Debug, derive_new::new)]
pub struct DefaultAggregator;

impl Aggregator for DefaultAggregator {
    fn aggregate(&self, predictions: &[LinkPrediction]) -> Nanosecs {
        predictions.iter().map(|p| p.delay).sum()
    }
}

#[derive(Debug, derive_new::new)]
pub struct ProportionalAggregator;

impl Aggregator for ProportionalAggregator {
    fn aggregate(&self, predictions: &[LinkPrediction]) -> Nanosecs {
        let loads = predictions
            .iter()
            .map(|p| p.offered_loads)
            .collect::<Vec<_>>();
        let nr_links = predictions.len();
        let nr_rounds = loads.iter().map(|a| a.len()).min().unwrap_or(0);
        let totals = loads.iter().map(|a| a.iter().sum()).collect::<Vec<f64>>();
        let mut adjusted = totals.clone();
        for round in 0..nr_rounds {
            let mut max_val = 0.0;
            let mut max_link = 0;
            for link in 0..nr_links {
                let val = loads[link][round];
                if loads[link][round] > max_val {
                    max_val = val;
                    max_link = link;
                }
            }
            for link in 0..nr_links {
                if link != max_link {
                    adjusted[link] -= loads[link][round];
                }
            }
        }
        itertools::izip!(predictions, adjusted, totals)
            .map(|(p, adj, tot)| p.delay.scale_by(adj / tot))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proportional_aggregator_correct() {
        let agg = ProportionalAggregator::new();
        let p1 = LinkPrediction {
            delay: Nanosecs::new(100),
            offered_loads: &[0.0, 1.0, 1.0],
        };
        let p2 = LinkPrediction {
            delay: Nanosecs::new(100),
            offered_loads: &[0.0, 0.5, 1.0],
        };
        assert_eq!(agg.aggregate(&[p1, p2]), Nanosecs::new(100));
    }
}
