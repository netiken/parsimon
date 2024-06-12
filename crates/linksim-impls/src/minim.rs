//! An interface to the Minim link-level simulator.

use std::cmp;

use parsimon_core::{
    constants::SZ_PKTHDR,
    linksim::{LinkSim, LinkSimError, LinkSimNodeKind, LinkSimResult, LinkSimSpec, LinkSimTopo},
    network::{FctRecord, FlowId, QIndex},
    units::{BitsPerSec, Bytes, Kilobytes, Mbps, Nanosecs},
};
use rustc_hash::{FxHashMap, FxHashSet};

/// A Minim link simulation.
#[derive(Debug, Clone, typed_builder::TypedBuilder, serde::Serialize, serde::Deserialize)]
pub struct MinimLink {
    /// The sending window.
    #[builder(default = Bytes::new(10_000), setter(into))]
    pub window: Bytes,
    /// DCTCP gain.
    #[builder(default = 0.0625)]
    pub dctcp_gain: f64,
    /// DCTCP additive increase.
    #[builder(default = Mbps::new(615).into(), setter(into))]
    pub dctcp_ai: BitsPerSec,
    /// Constant for computing DCTCP marking threshold.
    #[builder(default = 30, setter(into))]
    pub dctcp_marking_c: u64,
    /// Maximum packet size
    #[builder(default = Bytes::new(1000), setter(into))]
    pub sz_pktmax: Bytes,
    /// Switch weights.
    #[builder(setter(into), default = vec![Bytes::new(1024)])]
    pub quanta: Vec<Bytes>,
}

impl Default for MinimLink {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl LinkSim for MinimLink {
    fn name(&self) -> String {
        "minim".into()
    }

    fn simulate(&self, spec: LinkSimSpec) -> LinkSimResult {
        let cfg = self.build_config(spec)?;
        let records = minim::run(cfg).map_err(|e| anyhow::anyhow!(e))?;
        let records = records
            .into_iter()
            .map(|r| FctRecord {
                id: FlowId::new(r.id.into_usize()),
                size: Bytes::new(r.size.into_u64()),
                start: Nanosecs::new(r.start.into_u64()),
                qindex: QIndex::new(r.qindex.inner()),
                fct: Nanosecs::new(r.fct.into_u64()),
                ideal: Nanosecs::new(r.ideal.into_u64()),
            })
            .collect();

        Ok(records)
    }
}

impl MinimLink {
    fn build_config(&self, spec: LinkSimSpec) -> Result<minim::Config, LinkSimError> {
        let src_ids = spec
            .nodes
            .iter()
            .filter_map(|n| match n.kind {
                LinkSimNodeKind::Source => Some(n.id),
                _ => None,
            })
            .collect::<FxHashSet<_>>();
        let topo = LinkSimTopo::new(&spec);

        let srcs = src_ids
            .iter()
            .map(|&src| {
                let (delay2btl, link_rate) = if src == spec.bottleneck.from {
                    (Nanosecs::ZERO, spec.bottleneck.available_bandwidth)
                } else {
                    let path = topo.path(src, spec.bottleneck.from).unwrap();
                    (
                        path.iter().map(|l| l.delay).sum(),
                        path[0].available_bandwidth,
                    )
                };
                minim::SourceDesc::builder()
                    .id(minim::SourceId::new(src.inner()))
                    .delay2btl(minim::units::Nanosecs::new(delay2btl.into_u64()))
                    .link_rate(minim::units::BitsPerSec::new(link_rate.into_u64()))
                    .build()
            })
            .collect::<Vec<_>>();

        let mut src2dst2delay = FxHashMap::default();
        let mut max_qindex = QIndex::ZERO;
        let flows = spec
            .flows
            .into_iter()
            .map(|f| {
                let delay2dst = *src2dst2delay
                    .entry(f.src)
                    .or_insert_with(FxHashMap::default)
                    .entry(f.dst)
                    .or_insert_with(|| {
                        topo.path(f.src, f.dst)
                            .unwrap()
                            .iter()
                            .map(|l| l.delay)
                            .sum::<Nanosecs>()
                    });
                max_qindex = cmp::max(max_qindex, f.qindex);
                minim::FlowDesc {
                    id: minim::FlowId::new(f.id.inner()),
                    source: minim::SourceId::new(f.src.inner()),
                    qindex: minim::QIndex::new(f.qindex.inner()),
                    size: minim::units::Bytes::new(f.size.into_u64()),
                    start: minim::units::Nanosecs::new(f.start.into_u64()),
                    delay2dst: minim::units::Nanosecs::new(delay2dst.into_u64()),
                }
            })
            .collect::<Vec<_>>();

        if max_qindex.inner() >= self.quanta.len() {
            return Err(LinkSimError::Other(anyhow::anyhow!(
                "Invalid quanta for index {:?}",
                max_qindex
            )));
        }

        let marking_threshold = Kilobytes::new(
            spec.bottleneck
                .total_bandwidth
                .scale_by(10e9_f64.recip())
                .scale_by(self.dctcp_marking_c as f64)
                .into_u64(),
        );
        let bandwidth = if src_ids.contains(&spec.bottleneck.from) {
            spec.bottleneck.total_bandwidth.scale_by(100_f64)
        } else {
            spec.bottleneck.available_bandwidth
        };
        let quanta = self
            .quanta
            .iter()
            .map(|q| minim::units::Bytes::new(q.into_u64()))
            .collect();
        let cfg = minim::Config::builder()
            .bandwidth(minim::units::BitsPerSec::new(bandwidth.into_u64()))
            .quanta(quanta)
            .sources(srcs)
            .flows(flows)
            .window(minim::units::Bytes::new(self.window.into_u64()))
            .dctcp_marking_threshold(minim::units::Kilobytes::new(marking_threshold.into_u64()))
            .dctcp_gain(self.dctcp_gain)
            .dctcp_ai(minim::units::BitsPerSec::new(self.dctcp_ai.into_u64()))
            .sz_pktmax(minim::units::Bytes::new(self.sz_pktmax.into_u64()))
            .sz_pkthdr(minim::units::Bytes::new(SZ_PKTHDR.into_u64()))
            .build();
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests;
