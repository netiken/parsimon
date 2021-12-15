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
