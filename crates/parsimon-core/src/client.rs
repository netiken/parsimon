use std::{collections::HashMap, fmt::Display};

use crate::network::types::NodeId;

identifier!(FlowId, usize);
identifier!(ClientId, usize);
identifier!(VNodeId, usize);

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

impl Display for UniqFlowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.client(), self.flow())
    }
}

#[derive(Debug)]
pub struct VClient {
    pub id: ClientId,
    name: String,
    nodes: Vec<VNodeId>,
    flows: Vec<VFlow>,
}

impl VClient {
    /// Get a reference to the vclient's nodes.
    pub fn nodes(&self) -> &[VNodeId] {
        self.nodes.as_ref()
    }

    /// Get a reference to the vclient's flows.
    pub fn flows(&self) -> &[VFlow] {
        self.flows.as_ref()
    }
}

#[derive(Debug)]
pub struct VFlow {
    pub id: UniqFlowId,
    pub src: VNodeId,
    pub dst: VNodeId,
    pub size: u64,
    pub start: u64,
}

#[derive(Debug)]
pub(crate) struct Client {
    id: ClientId,
    name: String,
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

/// A mapping from clients to their node mappings
pub type ClientMap = HashMap<ClientId, NodeMap>;

/// A mapping from virtual nodes to physical nodes
pub type NodeMap = HashMap<VNodeId, NodeId>;
