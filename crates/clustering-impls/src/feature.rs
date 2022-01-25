use parsimon_core::{
    network::Flow,
    units::{Bytes, Nanosecs},
};

use crate::utils;

pub fn sz_arr_percentiles(flows: &[Flow]) -> Option<(Vec<Bytes>, Vec<Nanosecs>)> {
    (flows.len() >= 1000).then(|| {
        let sz = utils::percentiles(flows, |f| f.size);
        let arr = utils::percentiles(&utils::deltas(flows), |&x| x);
        (sz, arr)
    })
}
