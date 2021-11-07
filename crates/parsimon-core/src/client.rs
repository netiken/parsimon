use crate::network::types::NodeId;

identifier!(FlowId, usize);
identifier!(ClientId, usize);
identifier!(VNodeId, usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct UniqFlowId {
    inner: (ClientId, FlowId),
}

impl UniqFlowId {
    pub(crate) fn new(client: ClientId, flow: FlowId) -> Self {
        Self {
            inner: (client, flow),
        }
    }

    pub(crate) fn client(&self) -> ClientId {
        self.inner.0
    }

    pub(crate) fn flow(&self) -> FlowId {
        self.inner.1
    }
}

#[derive(Debug)]
pub struct VClient {
    id: ClientId,
    name: String,
    nr_nodes: usize,
    flows: Vec<VFlow>,
}

#[derive(Debug)]
pub(crate) struct VFlow {
    pub(crate) id: UniqFlowId,
    pub(crate) src: VNodeId,
    pub(crate) dst: VNodeId,
    pub(crate) size: u64,
    pub(crate) start: u64,
}

#[derive(Debug)]
pub(crate) struct Client {
    id: ClientId,
    name: String,
    nr_nodes: usize,
    flows: Vec<Flow>,
}

#[derive(Debug, Clone, Hash)]
pub(crate) struct Flow {
    pub(crate) id: UniqFlowId,
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
    pub(crate) size: u64,
    pub(crate) start: u64,
}
