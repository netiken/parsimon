use crate::ident::{ClientId, NodeId};

#[derive(Debug)]
pub struct Client {
    id: ClientId,
    name: String,
    nr_nodes: usize,
    flows: Vec<Flow>,
}

#[derive(Debug)]
pub struct Flow {
    pub client: ClientId,
    pub src: NodeId,
    pub dst: NodeId,
    pub size: u64,
}
