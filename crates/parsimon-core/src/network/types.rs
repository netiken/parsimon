use std::cmp::Ordering;

use crate::edist::EDistBuckets;
use crate::units::{Bytes, Gbps, Nanosecs};

pub const PKTSIZE_MAX: Bytes = Bytes::new(1000);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
}

impl Node {
    pub fn new_host(id: NodeId) -> Self {
        Self {
            id,
            kind: NodeKind::Host,
        }
    }

    pub fn new_switch(id: NodeId) -> Self {
        Self {
            id,
            kind: NodeKind::Switch,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NodeKind {
    Host,
    Switch,
}

identifier!(NodeId, usize);

/// A bidirectional channel.
#[derive(Debug, Clone, Copy, derive_new::new, serde::Serialize, serde::Deserialize)]
pub struct Link {
    pub a: NodeId,
    pub b: NodeId,
    pub bandwidth: Gbps,
    pub delay: Nanosecs,
}

impl Link {
    pub fn connects(&self, x: NodeId, y: NodeId) -> bool {
        self.a == x && self.b == y || self.a == y && self.b == x
    }
}

#[derive(Debug, PartialEq, Eq, derive_new::new, serde::Serialize)]
pub(crate) struct Channel {
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
    pub(crate) bandwidth: Gbps,
    pub(crate) delay: Nanosecs,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct TracedChannel {
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
    pub(crate) bandwidth: Gbps,
    pub(crate) delay: Nanosecs,
    pub(crate) flows: Vec<FlowId>,
}

impl TracedChannel {
    pub(crate) fn new_from(chan: &Channel) -> Self {
        Self {
            src: chan.src,
            dst: chan.dst,
            bandwidth: chan.bandwidth,
            delay: chan.delay,
            flows: Vec::new(),
        }
    }

    /// Get a reference to the traced channel's src.
    pub fn src(&self) -> NodeId {
        self.src
    }

    /// Get a reference to the traced channel's dst.
    pub fn dst(&self) -> NodeId {
        self.dst
    }

    /// Get an iterator over the traced channel's flow IDs
    pub fn flows(&self) -> impl Iterator<Item = FlowId> + '_ {
        self.flows.iter().copied()
    }

    delegate::delegate! {
        to self.flows {
            #[call(len)]
            pub fn nr_flows(&self) -> usize;

            #[call(push)]
            pub(crate) fn push_flow(&mut self, flow: FlowId);
        }
    }
}

#[derive(Debug)]
#[allow(unused)]
pub struct EDistChannel {
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
    pub(crate) bandwidth: Gbps,
    pub(crate) delay: Nanosecs,
    pub(crate) dists: EDistBuckets,
    pub(crate) offered_loads: Vec<f64>,
}

impl EDistChannel {
    pub(crate) fn new_from(chan: &TracedChannel) -> Self {
        Self {
            src: chan.src,
            dst: chan.dst,
            bandwidth: chan.bandwidth,
            delay: chan.delay,
            dists: EDistBuckets::new_empty(),
            offered_loads: Vec::new(),
        }
    }
}

identifier!(FlowId, usize);

#[derive(Debug, Default, Clone, Copy, Hash, serde::Serialize, serde::Deserialize)]
pub struct Flow {
    pub id: FlowId,
    pub src: NodeId,
    pub dst: NodeId,
    pub size: Bytes,
    pub start: Nanosecs,
}

#[derive(Debug, Clone, Copy)]
pub struct FctRecord {
    // From flow
    pub id: FlowId,
    pub size: Bytes,
    pub start: Nanosecs,
    // From simulation
    pub fct: Nanosecs,
    pub ideal: Nanosecs,
}

impl FctRecord {
    pub fn delay(&self) -> Nanosecs {
        // Work around kind-of-wrong ns-3 computation
        match self.fct.cmp(&self.ideal) {
            Ordering::Less | Ordering::Equal => Nanosecs::ZERO,
            Ordering::Greater => self.fct - self.ideal,
        }
    }

    pub fn pktnorm_delay(&self) -> f64 {
        let nr_pkts = (self.size.into_f64() / PKTSIZE_MAX.into_f64()).ceil();
        self.delay().into_f64() / nr_pkts
    }
}
