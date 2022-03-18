use parsimon_core::{
    linksim::{LinkSim, LinkSimError, LinkSimResult},
    network::{Channel, EdgeIndex, FctRecord, FlowId, SimNetwork},
    units::{BitsPerSec, Bytes, Kilobytes, Nanosecs},
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::utils;

#[derive(Debug, typed_builder::TypedBuilder)]
pub struct MinimLink {
    #[builder(setter(into))]
    window: Bytes,
    dctcp_gain: f64,
    #[builder(setter(into))]
    dctcp_ai: BitsPerSec,
}

impl LinkSim for MinimLink {
    fn simulate(&self, network: &SimNetwork, edge: EdgeIndex) -> LinkSimResult {
        let chan = network.edge(edge).ok_or(LinkSimError::UnknownEdge(edge))?;
        let flows = network.flows_on(edge).unwrap(); // we already know the channel exists

        let ack_rate = utils::ack_rate(network, edge);
        eprintln!("ack_rate = {:?}", ack_rate);

        let srcs = flows.iter().map(|f| f.src).collect::<FxHashSet<_>>();
        let (bsrc, bdst) = (chan.src(), chan.dst());

        let srcs = srcs
            .into_iter()
            .map(|src| {
                let (delay2btl, link_rate) = if src == bsrc {
                    let path = network.path(src, bdst, |c| c.first());
                    let link_rate = path.bandwidths().next().unwrap();
                    (Nanosecs::ZERO, link_rate)
                } else {
                    let path = network.path(src, bsrc, |c| c.first());
                    let link_rate = path.bandwidths().next().unwrap();
                    (path.delay(), link_rate)
                };
                minim::SourceDesc::builder()
                    .id(minim::SourceId::new(src.inner()))
                    .delay2btl(minim::units::Nanosecs::new(delay2btl.into_u64()))
                    .link_rate(minim::units::Gbps::new(link_rate.into_u64()))
                    .build()
            })
            .collect::<Vec<_>>();

        let mut src2dst2delay = FxHashMap::default();
        let flows = flows
            .into_iter()
            .map(|f| {
                let delay2dst = *src2dst2delay
                    .entry(f.src)
                    .or_insert_with(|| FxHashMap::default())
                    .entry(f.dst)
                    .or_insert_with(|| {
                        let path = network.path(f.src, f.dst, |c| c.first());
                        path.delay()
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

        let marking_threshold = Kilobytes::new(chan.bandwidth().scale_by(3.0).into_u64());
        let cfg = minim::Config::builder()
            .bandwidth(minim::units::Gbps::new(chan.bandwidth().into_u64()))
            .queue(minim::queue::FifoQ::new())
            .sources(srcs)
            .flows(flows)
            .window(minim::units::Bytes::new(self.window.into_u64()))
            .dctcp_marking_threshold(minim::units::Kilobytes::new(marking_threshold.into_u64()))
            .dctcp_gain(self.dctcp_gain)
            .dctcp_ai(minim::units::BitsPerSec::new(self.dctcp_ai.into_u64()))
            .build();

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
