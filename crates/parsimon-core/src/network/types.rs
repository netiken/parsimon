use crate::client::UniqFlowId;

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

/// A `Link` is a bidirectional channel
#[derive(Debug, Clone, Copy, derive_new::new)]
pub struct Link {
    pub a: NodeId,
    pub b: NodeId,
}

#[derive(Debug, PartialEq, Eq, derive_new::new, serde::Serialize)]
pub(crate) struct Channel {
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub(crate) struct TracedChannel {
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
    pub(crate) flows: Vec<UniqFlowId>,
}

impl TracedChannel {
    pub(crate) fn new_empty(chan: &Channel) -> Self {
        Self {
            src: chan.src,
            dst: chan.dst,
            flows: Vec::new(),
        }
    }

    delegate::delegate! {
        to self.flows {
            #[call(push)]
            pub(crate) fn push_flow(&mut self, flow: UniqFlowId);
        }
    }
}
