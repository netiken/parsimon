use std::collections::HashMap;

use crate::network::{Flow, NodeId, UniqFlowId};
use crate::units::{Bytes, Nanosecs};

identifier!(ClientId, usize);
identifier!(VNodeId, usize);

#[derive(Debug, derive_new::new)]
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

    /// Get a mutable reference to the vclient's flows.
    pub fn flows_mut(&mut self) -> &mut Vec<VFlow> {
        &mut self.flows
    }
}

#[derive(Debug)]
pub struct VFlow {
    pub id: UniqFlowId,
    pub src: VNodeId,
    pub dst: VNodeId,
    pub size: Bytes,
    pub start: Nanosecs,
}

#[derive(Debug)]
pub(crate) struct Client {
    id: ClientId,
    name: String,
    flows: Vec<Flow>,
}

/// A mapping from clients to their node mappings
pub type ClientMap = HashMap<ClientId, NodeMap>;

/// A mapping from virtual nodes to physical nodes
pub type NodeMap = HashMap<VNodeId, NodeId>;
