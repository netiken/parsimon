use crate::client::Flow;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, derive_new::new)]
pub(crate) struct Channel {
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
}

#[derive(Debug)]
pub(crate) struct TracedChannel {
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
    pub(crate) flows: Vec<Flow>,
}

impl TracedChannel {
    pub(crate) fn new(chan: Channel, flows: Vec<Flow>) -> Self {
        Self {
            src: chan.src,
            dst: chan.dst,
            flows,
        }
    }
}
