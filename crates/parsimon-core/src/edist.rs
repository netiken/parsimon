use std::ops::Range;

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

    pub fn bucket_ranges(&self) -> impl Iterator<Item = &Range<Bytes>> {
        self.inner.iter().map(|(range, _)| range)
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
    samples: Vec<f64>,
}

impl EDist {
    pub fn from_values(values: &[f64]) -> Result<Self, EDistError> {
        if values.is_empty() {
            return Err(EDistError::NoValues);
        }
        Ok(Self {
            samples: values.to_owned(),
        })
    }

    pub fn mean(&self) -> f64 {
        let total = self.samples.iter().sum::<f64>();
        total / self.samples.len() as f64
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EDistError {
    #[error("No values provided")]
    NoValues,
}

impl Distribution<f64> for EDist {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        self.samples.choose(rng).unwrap().to_owned()
    }
}
