use parsimon_core::{network::Flow, units::Nanosecs};

pub(crate) fn percentiles_100<T, U, F>(data: &[T], extract: F) -> Vec<U>
where
    U: Clone + Copy + PartialOrd + Ord,
    F: Fn(&T) -> U,
{
    assert!(!data.is_empty(), "percentiles: `data` is empty");
    let mut points = data.iter().map(|x| extract(x)).collect::<Vec<_>>();
    points.sort();
    let len = points.len();
    (0..100)
        .map(|p| {
            let i = ((p as f64 / 100.0) * len as f64).floor() as usize;
            points[i]
        })
        .collect()
}

// PRECONDITION: `a` cannot be all zeroes
pub(crate) fn wmape(a: &[f64], b: &[f64]) -> f64 {
    assert!(a.len() == b.len());
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x - y).abs())
        .sum::<f64>()
        / a.iter().sum::<f64>()
}

// PRECONDITION: the flows are sorted by start time
pub(crate) fn deltas(flows: &[Flow]) -> Vec<Nanosecs> {
    flows
        .windows(2)
        .map(|win| win[1].start - win[0].start)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::iter;

    use super::*;

    #[test]
    fn percentiles_sorts_and_indexes_correctly() {
        assert_eq!(
            percentiles_100(&[2, 1], |&x| x),
            iter::repeat(1)
                .take(50)
                .chain(iter::repeat(2).take(50))
                .collect::<Vec<_>>()
        )
    }

    #[test]
    fn wmape_correct() {
        let a = &[1., 2., 1., 2.];
        let b = &[2., 1., 2., 1.];
        let wmape = (wmape(a, b) * 100.).round() as u32;
        assert_eq!(wmape, 67);
    }
}
