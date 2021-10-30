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
#[derive(Debug, Clone, Copy)]
pub struct Link {
    pub a: NodeId,
    pub b: NodeId,
}

#[derive(Debug)]
pub(crate) struct Channel;
