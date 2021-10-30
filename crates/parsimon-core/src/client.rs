use crate::network::types::NodeId;

identifier!(ClientId, usize);
identifier!(VNodeId, usize);

#[derive(Debug)]
pub struct VClient {
    id: ClientId,
    name: String,
    nr_nodes: usize,
    flows: Vec<VFlow>,
}

#[derive(Debug)]
pub(crate) struct VFlow {
    pub(crate) client: ClientId,
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

#[derive(Debug)]
pub(crate) struct Flow {
    pub(crate) client: ClientId,
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
    pub(crate) size: u64,
    pub(crate) start: u64,
}
