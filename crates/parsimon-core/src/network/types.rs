use std::cmp::Ordering;

use petgraph::graph::EdgeIndex;

use crate::edist::EDistBuckets;
use crate::units::{BitsPerSec, Bytes, Nanosecs};

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
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Link {
    pub a: NodeId,
    pub b: NodeId,
    pub bandwidth: BitsPerSec,
    pub delay: Nanosecs,
}

impl Link {
    pub fn new(
        a: NodeId,
        b: NodeId,
        bandwidth: impl Into<BitsPerSec>,
        delay: impl Into<Nanosecs>,
    ) -> Self {
        Self {
            a,
            b,
            bandwidth: bandwidth.into(),
            delay: delay.into(),
        }
    }

    pub fn connects(&self, x: NodeId, y: NodeId) -> bool {
        self.a == x && self.b == y || self.a == y && self.b == x
    }
}

pub trait Channel {
    fn src(&self) -> NodeId;

    fn dst(&self) -> NodeId;

    fn bandwidth(&self) -> BitsPerSec;

    fn delay(&self) -> Nanosecs;
}

// All channels just copy these fields
macro_rules! channel_impl {
    ($name: ty) => {
        impl Channel for $name {
            fn src(&self) -> NodeId {
                self.src
            }

            fn dst(&self) -> NodeId {
                self.dst
            }

            fn bandwidth(&self) -> BitsPerSec {
                self.bandwidth
            }

            fn delay(&self) -> Nanosecs {
                self.delay
            }
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, derive_new::new, serde::Serialize)]
pub(crate) struct BasicChannel {
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
    pub(crate) bandwidth: BitsPerSec,
    pub(crate) delay: Nanosecs,
}

channel_impl!(BasicChannel);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FlowChannel {
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
    pub(crate) bandwidth: BitsPerSec,
    pub(crate) delay: Nanosecs,
    pub(crate) flows: Vec<FlowId>,
}

channel_impl!(FlowChannel);

impl FlowChannel {
    pub(crate) fn new_from(chan: &BasicChannel) -> Self {
        Self {
            src: chan.src,
            dst: chan.dst,
            bandwidth: chan.bandwidth,
            delay: chan.delay,
            flows: Vec::new(),
        }
    }

    /// Get an iterator over the traced channel's flow IDs
    pub fn flow_ids(&self) -> impl Iterator<Item = FlowId> + '_ {
        self.flows.iter().copied()
    }

    pub(crate) fn push_flow(&mut self, flow: &Flow) {
        self.flows.push(flow.id);
    }

    delegate::delegate! {
        to self.flows {
            #[call(len)]
            pub fn nr_flows(&self) -> usize;
        }
    }
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct EDistChannel {
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
    pub(crate) bandwidth: BitsPerSec,
    pub(crate) delay: Nanosecs,
    pub(crate) dists: EDistBuckets,
}

impl EDistChannel {
    pub(crate) fn new_from(chan: &FlowChannel) -> Self {
        Self {
            src: chan.src,
            dst: chan.dst,
            bandwidth: chan.bandwidth,
            delay: chan.delay,
            dists: EDistBuckets::new_empty(),
        }
    }
}

channel_impl!(EDistChannel);

#[derive(Debug)]
pub struct Path<'a, C> {
    inner: Vec<(EdgeIndex, &'a C)>,
}

impl<'a, C: Channel> Path<'a, C> {
    pub fn new(channels: Vec<(EdgeIndex, &'a C)>) -> Self {
        Self { inner: channels }
    }

    pub fn delay(&self) -> Nanosecs {
        self.inner.iter().map(|&(_, c)| c.delay()).sum()
    }

    pub fn bandwidths(&self) -> impl Iterator<Item = BitsPerSec> + '_ {
        self.inner.iter().map(|&(_, c)| c.bandwidth())
    }

    pub fn iter(&self) -> impl Iterator<Item = &(EdgeIndex, &'a C)> + '_ {
        self.inner.iter()
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
        // Some of these cases are possible because of rounding errors
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
