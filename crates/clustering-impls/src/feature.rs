use parsimon_core::{
    network::{types::FlowChannel, Channel, Flow},
    units::{Bytes, Nanosecs},
};

use crate::utils;

pub fn dists_and_load(chan: &FlowChannel, flows: &[Flow]) -> Option<DistsAndLoad> {
    (flows.len() >= 2).then(|| {
        let sizes = utils::percentiles(flows, |f| f.size);
        let deltas = utils::percentiles(&utils::deltas(flows), |&x| x);
        let nr_bytes = flows.iter().map(|f| f.size).sum::<Bytes>();
        let duration =
            flows.last().map(|f| f.start).unwrap() - flows.first().map(|f| f.start).unwrap();
        let bps = nr_bytes.into_f64() * 8.0 * 1e9 / duration.into_f64();
        let load = bps / chan.bandwidth().into_f64();
        DistsAndLoad {
            sizes,
            deltas,
            load,
        }
    })
}

#[derive(Debug, Clone)]
pub struct DistsAndLoad {
    pub sizes: Vec<Bytes>,
    pub deltas: Vec<Nanosecs>,
    pub load: f64,
}
