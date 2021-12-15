use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::network::Flow;
use crate::units::{Bytes, Gbps, Nanosecs};

pub(crate) fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

pub(crate) fn bdp(bandwidth: Gbps, delay: impl Into<Nanosecs>) -> Bytes {
    let bits_per_nanosec = bandwidth.into_f64();
    let bytes_per_nanosec = bits_per_nanosec / 8.0;
    let nanosecs = delay.into().into_f64();
    Bytes::new((bytes_per_nanosec * nanosecs).round() as u64)
}

pub(crate) fn offered_loads(
    bandwidth: Gbps,
    interval: impl Into<Nanosecs>,
    flows: &[Flow],
) -> Vec<f64> {
    let interval: Nanosecs = interval.into();
    let max_bytes = bdp(bandwidth, interval);
    let load = |bytes: Bytes| bytes.into_f64() / max_bytes.into_f64();
    let mut loads = Vec::new();
    let mut count = Bytes::ZERO;
    let mut next = interval;
    let mut push_load = |count: &mut Bytes, next: &mut Nanosecs| {
        let offered_bytes = std::cmp::min(max_bytes, *count);
        let load = load(offered_bytes);
        *count -= offered_bytes;
        *next += interval;
        loads.push(load);
    };
    for flow in flows {
        while flow.start >= next {
            push_load(&mut count, &mut next);
        }
        count += flow.size;
    }
    while count > Bytes::ZERO {
        push_load(&mut count, &mut next);
    }
    loads
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{Gigabytes, Microsecs};

    const BANDWIDTH: Gbps = Gbps::new(100);
    const INTERVAL: Microsecs = Microsecs::new(10);

    // Scale a slice of floats in [0, 1] to a slice of integers in [0, 100].
    fn integerify(vals: &[f64]) -> Vec<u32> {
        vals.iter()
            .map(|&val| (val * 100.0).round() as u32)
            .collect()
    }

    #[test]
    fn bdp_correct() {
        let bdp = bdp(BANDWIDTH, INTERVAL);
        eprintln!("bdp = {:?}", bdp);
        assert_eq!(bdp, Bytes::new(125_000));
    }

    #[test]
    fn offered_loads_time_advances() {
        let flows = &[
            Flow {
                size: Bytes::new(12_500),
                start: INTERVAL.into(),
                ..Default::default()
            },
            Flow {
                size: Bytes::new(12_500),
                start: (INTERVAL + INTERVAL).into(),
                ..Default::default()
            },
        ];
        let offered_loads = integerify(&offered_loads(BANDWIDTH, INTERVAL, flows));
        assert_eq!(offered_loads, vec![0, 10, 10]);
    }

    #[test]
    fn offered_loads_overflows_correctly() {
        let flow = Flow {
            size: Gigabytes::ONE.into(),
            start: INTERVAL.into(),
            ..Default::default()
        };
        let offered_loads = integerify(&offered_loads(BANDWIDTH, INTERVAL, &[flow]));
        assert_eq!(offered_loads[0], 0);
        let bdp = bdp(BANDWIDTH, INTERVAL);
        let nr_expected_ones = Into::<Bytes>::into(Gigabytes::ONE).into_u64() / bdp.into_u64();
        assert_eq!(offered_loads[1..].len(), nr_expected_ones as usize);
        assert!(offered_loads[1..].iter().all(|&load| load == 100));
    }
}
