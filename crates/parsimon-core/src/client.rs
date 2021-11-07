use std::fmt::Display;

use crate::network::types::NodeId;

identifier!(FlowId, usize);
identifier!(ClientId, usize);
identifier!(VNodeId, usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub(crate) struct UniqFlowId((ClientId, FlowId));

impl UniqFlowId {
    pub(crate) fn new(client: ClientId, flow: FlowId) -> Self {
        Self((client, flow))
    }

    pub(crate) fn client(&self) -> ClientId {
        self.0 .0
    }

    pub(crate) fn flow(&self) -> FlowId {
        self.0 .1
    }
}

impl Display for UniqFlowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.client(), self.flow())
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
