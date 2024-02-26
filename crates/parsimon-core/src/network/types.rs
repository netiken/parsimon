//! This module defines core types used to construct a network, such as [nodes](Node),
//! [links][Link], and [channels](Channel).

use std::cmp::Ordering;

use petgraph::graph::EdgeIndex;
use rustc_hash::FxHashSet;

use crate::constants::{SZ_ACK, SZ_PKTMAX};
use crate::edist::EDistBuckets;
use crate::units::{BitsPerSec, Bytes, Nanosecs};

/// A node in the network topology.
#[derive(Debug, Clone, PartialEq, Eq, derive_new::new, serde::Serialize, serde::Deserialize)]
pub struct Node {
    /// The node ID.
    pub id: NodeId,
    /// Whether the node is a host or a switch.
    pub kind: NodeKind,
}

impl Node {
    /// Create a new host with the given node ID.
    pub fn new_host(id: NodeId) -> Self {
        Self {
            id,
            kind: NodeKind::Host,
        }
    }

    /// Create a new switch with the given node ID.
    pub fn new_switch(id: NodeId) -> Self {
        Self {
            id,
            kind: NodeKind::Switch,
        }
    }
}

/// A node is either a host or a switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NodeKind {
    /// A host node.
    Host,
    /// A switch node.
    Switch,
}

identifier!(NodeId, usize);
impl NodeId {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}
/// A link is a bidirectional channel connecting two [nodes](Node).
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Link {
    /// The first endpoint.
    pub a: NodeId,
    /// The second endpoint.
    pub b: NodeId,
    /// The link bandwidth.
    pub bandwidth: BitsPerSec,
    /// The propagation delay.
    pub delay: Nanosecs,
}

impl Link {
    /// Creates a new link.
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

    /// Returns true if the given link connects nodes `x` and `y`.
    pub fn connects(&self, x: NodeId, y: NodeId) -> bool {
        self.a == x && self.b == y || self.a == y && self.b == x
    }
}

/// This trait defines routines that must be implemented by any channel in a topology.
pub trait Channel {
    /// The source node.
    fn src(&self) -> NodeId;

    /// The destination node.
    fn dst(&self) -> NodeId;

    /// The bandwidth.
    fn bandwidth(&self) -> BitsPerSec;

    /// The propagation delay.
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

impl<T: Channel> Channel for &T {
    fn src(&self) -> NodeId {
        (*self).src()
    }

    fn dst(&self) -> NodeId {
        (*self).dst()
    }

    fn bandwidth(&self) -> BitsPerSec {
        (*self).bandwidth()
    }

    fn delay(&self) -> Nanosecs {
        (*self).delay()
    }
}

/// A `BasicChannel` is a one-way channel between two nodes with some bandwidth and delay.
#[derive(Debug, Clone, PartialEq, Eq, derive_new::new, serde::Serialize)]
pub struct BasicChannel {
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
    pub(crate) bandwidth: BitsPerSec,
    pub(crate) delay: Nanosecs,
}

channel_impl!(BasicChannel);

/// A `FlowChannel` is a channel containing flows to simulate.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FlowChannel {
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
    pub(crate) bandwidth: BitsPerSec,
    pub(crate) delay: Nanosecs,

    // `FlowChannel` specific data
    pub(crate) nr_bytes: Bytes,
    pub(crate) nr_ack_bytes: Bytes,
    pub(crate) flow_srcs: FxHashSet<NodeId>,
    pub(crate) flow_dsts: FxHashSet<NodeId>,
    pub(crate) flow_start: Nanosecs,
    pub(crate) flow_end: Nanosecs,
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
            nr_bytes: Bytes::ZERO,
            nr_ack_bytes: Bytes::ZERO,
            flow_srcs: FxHashSet::default(),
            flow_dsts: FxHashSet::default(),
            flow_start: Nanosecs::MAX,
            flow_end: Nanosecs::ZERO,
            flows: Vec::new(),
        }
    }

    /// Get an iterator over the traced channel's flow IDs
    pub fn flow_ids(&self) -> impl Iterator<Item = FlowId> + '_ {
        self.flows.iter().copied()
    }

    pub(crate) fn push_flow(&mut self, flow: &Flow) {
        self.nr_bytes += flow.size;
        let nr_pkts = (flow.size.into_f64() / SZ_PKTMAX.into_f64()).ceil();
        let nr_ack_bytes = SZ_ACK.scale_by(nr_pkts);
        self.nr_ack_bytes += nr_ack_bytes;
        self.flow_srcs.insert(flow.src);
        self.flow_dsts.insert(flow.dst);
        self.flow_start = std::cmp::min(self.flow_start, flow.start);
        self.flow_end = std::cmp::max(self.flow_end, flow.start);
        self.flows.push(flow.id);
    }
    
    pub(crate) fn duration(&self) -> Nanosecs {
        if self.flows.is_empty() {
            Nanosecs::ZERO
        } else {
            self.flow_end - self.flow_start
        }
    }

    delegate::delegate! {
        to self.flows {
            /// Returns the number of flows traversing this channel.
            #[call(len)]
            pub fn nr_flows(&self) -> usize;
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EDistChannel {
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

/// A `Path` is a sequence of channels.
#[derive(Debug)]
pub struct Path<'a, C> {
    inner: Vec<(EdgeIndex, &'a C)>,
}

impl<'a, C: Channel> Path<'a, C> {
    pub(crate) fn new(channels: Vec<(EdgeIndex, &'a C)>) -> Self {
        Self { inner: channels }
    }

    /// Returns the propagation delay along the entire path.
    pub fn delay(&self) -> Nanosecs {
        self.inner.iter().map(|&(_, c)| c.delay()).sum()
    }

    /// Returns an iterator over the link bandwidths in the path.
    pub fn bandwidths(&self) -> impl Iterator<Item = BitsPerSec> + '_ {
        self.inner.iter().map(|&(_, c)| c.bandwidth())
    }

    /// Returns an iterator over the edge indices in the path.
    pub fn iter(&self) -> impl Iterator<Item = &(EdgeIndex, &'a C)> + '_ {
        self.inner.iter()
    }
}

identifier!(FlowId, usize);
impl FlowId {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}
/// A flow is a logically grouped sequence of bytes from a source to a destination.
#[derive(Debug, Default, Clone, Copy, Hash, serde::Serialize, serde::Deserialize)]
pub struct Flow {
    /// The flow ID.
    pub id: FlowId,
    /// The flow source.
    pub src: NodeId,
    /// The flow destination.
    pub dst: NodeId,
    /// The flow size.
    pub size: Bytes,
    /// The flow's start time.
    pub start: Nanosecs,
}
impl Flow {
    pub fn get_ids(&self) -> Vec<usize> {
        vec![self.id.0, self.src.0, self.dst.0]
    }
}

/// An `FctRecord` records the flow completion time of a particular flow.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct FctRecord {
    /// The flow ID.
    pub id: FlowId,
    /// The flow size.
    pub size: Bytes,
    /// The flow's start time.
    pub start: Nanosecs,

    /// The measured flow completion time.
    pub fct: Nanosecs,
    /// The ideal flow completion time on an unloaded network.
    pub ideal: Nanosecs,
}

impl FctRecord {
    /// Returns the delay encountered by the flow, defined as the measured FCT minus the ideal FCT.
    pub fn delay(&self) -> Nanosecs {
        // Some of these cases are possible because of rounding errors
        match self.fct.cmp(&self.ideal) {
            Ordering::Less | Ordering::Equal => Nanosecs::ZERO,
            Ordering::Greater => self.fct - self.ideal,
        }
    }

    /// Returns the packet-normalized delay, which is the delay normalized by the number of packets
    /// in the flow.
    pub fn pktnorm_delay(&self) -> f64 {
        let nr_pkts = (self.size.into_f64() / SZ_PKTMAX.into_f64()).ceil();
        self.delay().into_f64() / nr_pkts
    }

    /// Returns the FCT slowdown which is the measured FCT divided by the ideal FCT.
    pub fn slowdown(&self) -> f64 {
        self.fct.into_f64() / self.ideal.into_f64()
    }
}
