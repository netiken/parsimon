use std::collections::HashSet;

use crate::{
    client::{ClientId, ClientMap, UniqFlowId, VClient, VFlow, VNodeId},
    network::Network,
};

#[derive(Debug)]
pub struct Spec {
    pub network: Network,
    pub clients: Vec<VClient>,
    pub mappings: ClientMap,
}

impl Spec {
    /// Validate a specification.
    ///
    /// Correctness properties:
    ///
    /// - For every client `c`, virtual flows in `c.flows` cannot reference more virtual nodes than
    ///   specified in `c.nr_nodes`.
    /// - Every client must have an entry in `ClientMap`.
    /// 
    /// TODO (next)
    pub(crate) fn validate(&self) -> Result<(), SpecError> {
        // CORRECTNESS: For every client `c`, virtual flows in `c.flows` cannot reference more
        // virtual nodes than specified in `c.nr_nodes`.
        for client in &self.clients {
            let vnodes = client.nodes().iter().copied().collect::<HashSet<_>>();
            for &VFlow { id, src, dst, .. } in client.flows() {
                if !vnodes.contains(&src) {
                    return Err(SpecError::InvalidFlowSrc { flow: id, src });
                }
                if !vnodes.contains(&dst) {
                    return Err(SpecError::InvalidFlowDst { flow: id, dst });
                }
            }
        }
        // TODO (next)
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SpecError {
    #[error("flow {flow} has an invalid source ({src})")]
    InvalidFlowSrc { flow: UniqFlowId, src: VNodeId },

    #[error("flow {flow} has an invalid source ({dst})")]
    InvalidFlowDst { flow: UniqFlowId, dst: VNodeId },

    #[error("client {0} does not have a mapping")]
    MissingClientMapping(ClientId),

    #[error("client {client} has no VNode {from}")]
    InvalidMappingFrom { client: ClientId, from: VNodeId },

    #[error("client {client} has no VNode {to}")]
    InvalidMappingTo { client: ClientId, to: VNodeId },
}
