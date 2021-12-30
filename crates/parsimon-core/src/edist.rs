use std::{collections::BTreeMap, ops::Range};

use ordered_float::OrderedFloat;
use rand::prelude::*;

use crate::units::Bytes;

#[derive(Debug, Clone)]
pub struct EDistBuckets {
    inner: Vec<(Range<Bytes>, EDist)>,
}

impl EDistBuckets {
    pub(crate) fn new_empty() -> Self {
        Self { inner: Vec::new() }
    }

    pub(crate) fn fill<T, F, G>(&mut self, data: &[T], f: F, g: G) -> Result<(), EDistError>
    where
        T: Clone + Copy,   // a datatype from which a size and a sample can be extracted
        F: Fn(T) -> Bytes, // size extractor
        G: Fn(T) -> f64,   // sample extractor
    {
        let buckets = bucket(data, f);
        let inner = buckets
            .into_iter()
            .map(|(bkt, data)| {
                let samples = data.into_iter().map(|x| g(x)).collect::<Vec<_>>();
                let dist = EDist::from_values(&samples)?;
                Ok((bkt, dist))
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.inner = inner;
        Ok(())
    }

    pub fn for_size(&self, size: Bytes) -> Option<&EDist> {
        self.inner
            .iter()
            .find_map(|(bkt, dist)| bkt.contains(&size).then(|| dist))
    }
}

// Bucket data automatically such that for each bucket `B`,
//
// 1. `B.len() >= 100`
// 2. `B.max() >= 2 * B.min()`
fn bucket<T, F>(data: &[T], f: F) -> Vec<(Range<Bytes>, Vec<T>)>
where
    T: Clone + Copy,
    F: Fn(T) -> Bytes,
{
    let mut data = Vec::from(data);
    data.sort_by(|&a, &b| f(a).cmp(&f(b)));
    let mut buckets = Vec::new();
    let mut acc = Vec::new();
    for datum in data {
        let min = acc.first().map(|&x| f(x)).unwrap_or(Bytes::MAX);
        let max = f(datum);
        acc.push(datum);
        if min <= max.scale_by(0.5) && acc.len() >= 100 {
            buckets.push((min..max + Bytes::ONE, acc.clone()));
            acc.clear();
        }
    }
    if !acc.is_empty() {
        // Any elements left over in `acc`, however few, will be placed in the last bucket
        let min = f(acc[0]);
        let max = f(acc[acc.len() - 1]);
        buckets.push((min..max + Bytes::ONE, acc));
    }
    buckets
}

#[derive(Debug, Clone)]
pub struct EDist {
    ecdf: Vec<(f64, f64)>,
}

impl EDist {
    pub fn from_ecdf(ecdf: Vec<(f64, f64)>) -> Result<Self, EDistError> {
        if ecdf.is_empty() {
            return Err(EDistError::InvalidEcdf);
        }
        let len = ecdf.len();
        if (ecdf[len - 1].1 - 100.0).abs() > f64::EPSILON {
            return Err(EDistError::InvalidEcdf);
        }
        for i in 1..len {
            if ecdf[i].1 <= ecdf[i - 1].1 || ecdf[i].0 <= ecdf[i - 1].0 {
                return Err(EDistError::InvalidEcdf);
            }
        }
        Ok(Self { ecdf })
    }

    pub fn from_values(values: &[f64]) -> Result<Self, EDistError> {
        if values.is_empty() {
            return Err(EDistError::NoValues);
        }
        let mut values = values
            .iter()
            .map(|&val| OrderedFloat(val))
            .collect::<Vec<_>>();
        values.sort();
        let points = values
            .iter()
            .enumerate()
            .map(|(i, &size)| (size, (i + 1) as f64 / values.len() as f64))
            .collect::<Vec<_>>();
        let mut map = BTreeMap::new();
        for (x, y) in points {
            // Updates if key exists, kicking out the old value
            map.insert(x, y);
        }
        let ecdf = map
            .into_iter()
            .map(|(x, y)| (x.into_inner(), y * 100.0))
            .collect();
        Self::from_ecdf(ecdf)
    }

    pub fn mean(&self) -> f64 {
        let mut s = 0.0;
        let (mut last_x, mut last_y) = self.ecdf[0];
        for &(x, y) in self.ecdf.iter().skip(1) {
            s += (x + last_x) / 2.0 * (y - last_y);
            last_x = x;
            last_y = y;
        }
        s / 100.0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EDistError {
    #[error("EDist is invalid")]
    InvalidEcdf,

    #[error("No values provided")]
    NoValues,
}

impl Distribution<f64> for EDist {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        let y = rng.gen_range(0.0..=100.0);
        let mut i = 0;
        while y > self.ecdf[i].1 {
            i += 1;
        }
        match i {
            0 => self.ecdf[0].0,
            _ => {
                let (x0, y0) = self.ecdf[i - 1];
                let (x1, y1) = self.ecdf[i];
                x0 + (x1 - x0) / (y1 - y0) * (y - y0)
            }
        }
    }
}
