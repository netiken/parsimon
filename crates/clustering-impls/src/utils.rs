//! Helper utilities.

use parsimon_core::{network::Flow, units::Nanosecs};

/// Returns 1000 quantiles from `data` using `extract` to select values to compare.
pub fn percentiles<T, U, F>(data: &[T], extract: F) -> Vec<U>
where
    U: Clone + Copy + PartialOrd + Ord,
    F: Fn(&T) -> U,
{
    assert!(!data.is_empty(), "percentiles: `data` is empty");
    let mut points = data.iter().map(extract).collect::<Vec<_>>();
    points.sort();
    let len = points.len();
    (0..1000)
        .map(|p| {
            let i = ((p as f64 / 1000.0) * len as f64).floor() as usize;
            points[i]
        })
        .collect()
}

/// Returns the inter-arrival times of a given list of flows.
/// PRECONDITION: the flows are sorted by start time.
pub fn deltas(flows: &[Flow]) -> Vec<Nanosecs> {
    let nr_flows = flows.len();
    assert!(
        nr_flows >= 2,
        "deltas: `flows` not long enough, `nr_flows` = {nr_flows}",
    );
    flows
        .windows(2)
        .map(|win| win[1].start - win[0].start)
        .collect()
}

/// Rescales the data in `a` to [0, 1].
pub fn rescale<T>(a: &[T]) -> Vec<f64>
where
    T: Clone + Copy + Into<f64>,
{
    assert!(!a.is_empty(), "rescale: `a` is empty");
    let cmp = |x: f64, y: f64| x.partial_cmp(&y).expect("rescale: floating point error");
    let iter = a.iter().map(|&x| Into::<f64>::into(x));
    let min = iter.clone().min_by(|&x, &y| cmp(x, y)).unwrap();
    let max = iter.clone().max_by(|&x, &y| cmp(x, y)).unwrap();
    let range = max - min;
    iter.map(|x| (x - min) / range).collect()
}

/// Weighted mean absolute percentage error (WMAPE)
/// PRECONDITION: `a` cannot be all zeroes.
pub fn wmape<T>(a: &[T], b: &[T]) -> f64
where
    T: Clone + Copy + Into<f64>,
{
    assert!(a.len() == b.len());
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x.into() - y.into()).abs())
        .sum::<f64>()
        / a.iter().map(|&x| Into::<f64>::into(x).abs()).sum::<f64>()
}

/// Mean absolute error.
pub fn mae<T>(a: &[T], b: &[T]) -> f64
where
    T: Clone + Copy + Into<f64>,
{
    assert!(a.len() == b.len());
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x.into() - y.into()).abs())
        .sum::<f64>()
        / a.len() as f64
}

#[cfg(test)]
mod tests {
    use std::iter;

    use super::*;

    #[test]
    fn percentiles_sorts_and_indexes_correctly() {
        assert_eq!(
            percentiles(&[2, 1], |&x| x),
            iter::repeat(1)
                .take(500)
                .chain(iter::repeat(2).take(500))
                .collect::<Vec<_>>()
        )
    }

    #[test]
    fn rescale_correct() {
        let a = [0., 25., 50., 100.];
        assert_eq!(rescale(&a), vec![0., 0.25, 0.5, 1.0]);
    }

    #[test]
    fn mae_correct() {
        let a = &[1., 2., 1., 2.];
        let b = &[2., 1., 2., 1.];
        let mae = (mae(a, b) * 100.).round() as u32;
        assert_eq!(mae, 100);
    }

    #[test]
    fn wmape_correct() {
        let a = &[1., 2., 1., 2.];
        let b = &[2., 1., 2., 1.];
        let wmape = (wmape(a, b) * 100.).round() as u32;
        assert_eq!(wmape, 67);
    }
}
