#![allow(unused)]

use std::cmp;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use rayon::prelude::*;

use crate::network::{Channel, Flow};
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

pub(crate) fn par_chunks<T, F, R>(data: &[T], f: F) -> impl Iterator<Item = R>
where
    T: Sync,
    R: Send,
    F: Fn(&[T]) -> Vec<R> + Sync,
{
    let (s, r) = crossbeam_channel::unbounded();
    let nr_cpus = num_cpus::get();
    let nr_elems = data.len();
    let chunk_size = std::cmp::max(nr_elems / nr_cpus, 1);
    data.chunks(chunk_size)
        .par_bridge()
        .for_each_with(s, |s, chunk| {
            let v = f(chunk);
            s.send(v).unwrap(); // channel will not become disconnected
        });
    r.into_iter().map(|v| v.into_iter()).flatten()
}

const SZ_PKTMAX: Bytes = Bytes::new(1_000);
const SZ_HDR: Bytes = Bytes::new(48);

pub(crate) fn ideal_fct<T>(size: Bytes, hops: &[T]) -> Nanosecs
where
    T: Channel,
{
    assert!(!hops.is_empty());
    let bandwidths = hops.iter().map(|c| c.bandwidth()).collect::<Vec<_>>();
    let min_bw = bandwidths.iter().min().unwrap();
    let sz_head_ = cmp::min(SZ_PKTMAX, size);
    let sz_head = (sz_head_ != Bytes::ZERO)
        .then(|| sz_head_ + SZ_HDR)
        .unwrap_or(Bytes::ZERO);
    let sz_rest_ = size - sz_head_;
    let head_delay = bandwidths
        .iter()
        .map(|bw| bw.length(sz_head))
        .sum::<Nanosecs>();
    let rest_delay = {
        let nr_full_pkts = sz_rest_.into_usize() / SZ_PKTMAX.into_usize();
        let sz_full_pkt = SZ_PKTMAX + SZ_HDR;
        let sz_partial_pkt_ = Bytes::new(sz_rest_.into_u64() % SZ_PKTMAX.into_u64());
        let sz_partial_pkt = (sz_partial_pkt_ != Bytes::ZERO)
            .then(|| sz_partial_pkt_ + SZ_HDR)
            .unwrap_or(Bytes::ZERO);
        min_bw.length(sz_full_pkt).scale_by(nr_full_pkts as f64) + min_bw.length(sz_partial_pkt)
    };
    let prop_delay = hops.iter().map(|c| c.delay()).sum::<Nanosecs>();
    head_delay + rest_delay + prop_delay
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
