use crate::{client::VClient, mapping::ClientMap, network::Network};

#[derive(Debug)]
pub struct Spec {
    pub network: Network,
    pub clients: Vec<VClient>,
    pub mappings: ClientMap,
}
