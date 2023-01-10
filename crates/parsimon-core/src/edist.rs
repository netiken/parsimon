//! Types for building empirical distributions

use std::{collections::VecDeque, ops::Range};

use rand::prelude::*;

use crate::units::Bytes;

/// Empirical distributions bucketed by size ranges (in bytes).
#[derive(Debug, Clone)]
pub struct EDistBuckets {
    inner: Vec<(Range<Bytes>, EDist)>,
}

impl EDistBuckets {
    pub(crate) fn new_empty() -> Self {
        Self {
            inner: vec![(Bytes::ZERO..Bytes::MAX, EDist::new())],
        }
    }

    pub(crate) fn fill<T, F, G>(
        &mut self,
        data: &[T],
        f: F,
        mut g: G,
        opts: BucketOpts,
    ) -> Result<(), EDistError>
    where
        T: Clone + Copy,   // a datatype from which a size and a sample can be extracted
        F: Fn(T) -> Bytes, // size extractor
        G: Fn(T) -> f64,   // sample extractor
    {
        let buckets = bucket(data, f, &opts);
        let inner = buckets
            .into_iter()
            .map(|(bkt, data)| {
                let samples = data.into_iter().map(&mut g).collect::<Vec<_>>();
                let dist = EDist::from_values(&samples)?;
                Ok((bkt, dist))
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.inner = inner;
        Ok(())
    }

    /// Returns an iterator over all size ranges.
    pub fn bucket_ranges(&self) -> impl Iterator<Item = &Range<Bytes>> {
        self.inner.iter().map(|(range, _)| range)
    }

    /// Returns the empirical distribution for a particular size.
    pub fn for_size(&self, size: Bytes) -> Option<&EDist> {
        self.inner
            .iter()
            .find_map(|(bkt, dist)| bkt.contains(&size).then_some(dist))
    }
}

/// Parameters for the bucketing algorithm.
#[derive(Debug, Clone, Copy, derive_new::new)]
pub struct BucketOpts {
    /// For each bucket `B`, `B.max() >= x * B.min()`.
    pub x: u8,
    /// For each bucket `B`, `B.max() >= b`.
    pub b: usize,
}

impl Default for BucketOpts {
    fn default() -> Self {
        Self { x: 2, b: 100 }
    }
}

// Bucket data automatically such that for each bucket `B`,
//
// 1. `B.len() >= opts.b`
// 2. `B.max() >= opts.x * B.min()`
fn bucket<T, F>(data: &[T], f: F, opts: &BucketOpts) -> Vec<(Range<Bytes>, Vec<T>)>
where
    T: Clone + Copy,
    F: Fn(T) -> Bytes,
{
    let mut data = Vec::from(data);
    data.sort_by(|&a, &b| f(a).cmp(&f(b)));
    let mut data = VecDeque::from(data);
    let mut buckets = Vec::new();
    let mut acc = Vec::new();
    let mut acc_min = Bytes::ZERO;
    let mut acc_max;
    while let Some(datum) = data.pop_front() {
        acc.push(datum);
        acc_max = f(datum);
        if acc_min <= acc_max.scale_by((opts.x as f64).recip()) && acc.len() >= opts.b {
            while let Some(&datum) = data.front() {
                if f(datum) == acc_max {
                    acc.push(data.pop_front().unwrap());
                } else {
                    break;
                }
            }
            buckets.push((acc_min..acc_max + Bytes::ONE, acc.clone()));
            acc.clear();
            acc_min = acc_max + Bytes::ONE;
        }
    }
    if !acc.is_empty() {
        // Any elements left over in `acc`, however few, will be placed in the last bucket
        buckets.push((acc_min..Bytes::MAX, acc));
    }
    buckets
}

/// An empirical distribution.
#[derive(Debug, Clone, derive_new::new)]
pub struct EDist {
    #[new(default)]
    samples: Vec<f64>,
}

impl EDist {
    /// Creates a new empirical distribution from a slice of values.
    pub fn from_values(values: &[f64]) -> Result<Self, EDistError> {
        if values.is_empty() {
            return Err(EDistError::NoValues);
        }
        Ok(Self {
            samples: values.to_owned(),
        })
    }

    /// Returns the mean of the distribution.
    pub fn mean(&self) -> f64 {
        let total = self.samples.iter().sum::<f64>();
        total / self.samples.len() as f64
    }
}

/// Error type for creating empirical distributions.
#[derive(Debug, thiserror::Error)]
pub enum EDistError {
    /// No values were provided---cannot have an empty distribution.
    #[error("No values provided")]
    NoValues,
}

impl Distribution<f64> for EDist {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        self.samples.choose(rng).unwrap_or(&0_f64).to_owned()
    }
}
