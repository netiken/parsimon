//! An interface to the Minim link-level simulator.

use parsimon_core::{
    constants::{SZ_PKTHDR, SZ_PKTMAX},
    linksim::{LinkSim, LinkSimError, LinkSimNodeKind, LinkSimResult, LinkSimSpec, LinkSimTopo},
    network::{FctRecord, FlowId},
    units::{BitsPerSec, Bytes, Kilobytes, Nanosecs},
};
use rustc_hash::{FxHashMap, FxHashSet};

/// A Minim link simulation.
#[derive(Debug, typed_builder::TypedBuilder, serde::Serialize, serde::Deserialize)]
pub struct MinimLink {
    /// The sending window.
    #[builder(setter(into))]
    pub window: Bytes,
    /// DCTCP gain.
    pub dctcp_gain: f64,
    /// DCTCP additive increase.
    #[builder(setter(into))]
    pub dctcp_ai: BitsPerSec,
    /// DCTCP masking threshold
    #[builder(default = 30.0)]
    pub dctcp_k: f64,
}

impl LinkSim for MinimLink {
    fn name(&self) -> String {
        "minim".into()
    }

    fn simulate(&self, spec: LinkSimSpec) -> LinkSimResult {
        let cfg = self.build_config(spec)?;
        let records = minim::run(cfg);
        let records = records
            .into_iter()
            .map(|r| FctRecord {
                id: FlowId::new(r.id.into_usize()),
                size: Bytes::new(r.size.into_u64()),
                start: Nanosecs::new(r.start.into_u64()),
                fct: Nanosecs::new(r.fct.into_u64()),
                ideal: Nanosecs::new(r.ideal.into_u64()),
            })
            .collect();

        Ok(records)
    }
}

impl MinimLink {
    fn build_config(
        &self,
        spec: LinkSimSpec,
    ) -> Result<minim::Config<minim::queue::FifoQ>, LinkSimError> {
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
                minim::FlowDesc {
                    id: minim::FlowId::new(f.id.inner()),
                    source: minim::SourceId::new(f.src.inner()),
                    size: minim::units::Bytes::new(f.size.into_u64()),
                    start: minim::units::Nanosecs::new(f.start.into_u64()),
                    delay2dst: minim::units::Nanosecs::new(delay2dst.into_u64()),
                }
            })
            .collect::<Vec<_>>();

        let marking_threshold = Kilobytes::new(
            spec.bottleneck
                .total_bandwidth
                .scale_by(1e9_f64.recip())
                // .scale_by(3_f64)
                .scale_by(self.dctcp_k/10.0)
                .into_u64(),
        );
        let bandwidth = if src_ids.contains(&spec.bottleneck.from) {
            spec.bottleneck.total_bandwidth.scale_by(100_f64)
        } else {
            spec.bottleneck.available_bandwidth
        };
        let cfg = minim::Config::builder()
            .bandwidth(minim::units::BitsPerSec::new(bandwidth.into_u64()))
            .queue(minim::queue::FifoQ::new())
            .sources(srcs)
            .flows(flows)
            .window(minim::units::Bytes::new(self.window.into_u64()))
            .dctcp_marking_threshold(minim::units::Kilobytes::new(marking_threshold.into_u64()))
            .dctcp_gain(self.dctcp_gain)
            .dctcp_ai(minim::units::BitsPerSec::new(self.dctcp_ai.into_u64()))
            .sz_pktmax(minim::units::Bytes::new(SZ_PKTMAX.into_u64()))
            .sz_pkthdr(minim::units::Bytes::new(SZ_PKTHDR.into_u64()))
            .build();
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests;
