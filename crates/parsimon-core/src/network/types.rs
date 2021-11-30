use crate::client::ClientId;
use crate::edist::EDistBuckets;
use crate::units::{Bytes, Gbps, Nanosecs};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum NodeKind {
    Host,
    Switch,
}

identifier!(NodeId, usize);

/// A bidirectional channel.
#[derive(Debug, Clone, Copy, derive_new::new)]
pub struct Link {
    pub a: NodeId,
    pub b: NodeId,
    pub bandwidth: Gbps,
    pub delay: Nanosecs,
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
    pub(crate) flows: Vec<UniqFlowId>,
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
    pub fn flows(&self) -> impl Iterator<Item = UniqFlowId> + '_ {
        self.flows.iter().copied()
    }

    delegate::delegate! {
        to self.flows {
            #[call(len)]
            pub fn nr_flows(&self) -> usize;

            #[call(push)]
            pub(crate) fn push_flow(&mut self, flow: UniqFlowId);
        }
    }
}

#[derive(Debug)]
pub struct EDistChannel {
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
    pub(crate) bandwidth: Gbps,
    pub(crate) delay: Nanosecs,
    pub(crate) dists: EDistBuckets,
}

impl EDistChannel {
    pub(crate) fn new_from(chan: &TracedChannel) -> Self {
        Self {
            src: chan.src,
            dst: chan.dst,
            bandwidth: chan.bandwidth,
            delay: chan.delay,
            dists: EDistBuckets::new_empty(),
        }
    }
}

identifier!(FlowId, usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub struct UniqFlowId((ClientId, FlowId));

impl UniqFlowId {
    pub fn new(client: ClientId, flow: FlowId) -> Self {
        Self((client, flow))
    }

    pub fn client(&self) -> ClientId {
        self.0 .0
    }

    pub fn flow(&self) -> FlowId {
        self.0 .1
    }
}

impl std::fmt::Display for UniqFlowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.client(), self.flow())
    }
}

#[derive(Debug, Clone, Copy, Hash)]
pub struct Flow {
    pub id: UniqFlowId,
    pub src: NodeId,
    pub dst: NodeId,
    pub size: Bytes,
    pub start: Nanosecs,
}

#[derive(Debug, Clone, Copy)]
pub struct FctRecord {
    // From flow
    pub id: UniqFlowId,
    pub src: NodeId,
    pub dst: NodeId,
    pub size: Bytes,
    pub start: Nanosecs,
    // From simulation
    pub end: Nanosecs,
    pub ideal: Nanosecs,
}
