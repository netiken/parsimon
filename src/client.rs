identifier!(ClientId, usize);
identifier!(VNodeId, usize);
identifier!(NodeId, usize);

#[derive(Debug)]
pub struct VClient {
    id: ClientId,
    name: String,
    nr_nodes: usize,
    flows: Vec<VFlow>,
}

#[derive(Debug)]
pub struct VFlow {
    pub client: ClientId,
    pub src: VNodeId,
    pub dst: VNodeId,
    pub size: u64,
}

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
