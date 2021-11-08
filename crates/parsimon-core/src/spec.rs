use std::collections::HashSet;

use crate::{
    client::{ClientId, ClientMap, UniqFlowId, VClient, VFlow, VNodeId},
    network::{types::NodeId, Network},
};

#[derive(Debug)]
pub struct Spec {
    network: Network,
    clients: Vec<VClient>,
    mappings: ClientMap,
}

/// A `ValidSpec` is exactly the same thing as a `Spec`, except it can only be created through a
/// call to `Spec::validate`, and it has public fields.
#[derive(Debug)]
pub(crate) struct ValidSpec {
    pub(crate) network: Network,
    pub(crate) clients: Vec<VClient>,
    pub(crate) mappings: ClientMap,
}

impl Spec {
    /// Validate a specification, producing a `ValidSpec`.
    ///
    /// Correctness properties:
    ///
    /// - Every flow must have a valid source and destination.
    /// - Every client must have an entry in `ClientMap`.
    /// - Every mapping must be from a valid virtual node to a valid physical node.
    //
    // TODO (next): Test me
    pub(crate) fn validate(self) -> Result<ValidSpec, SpecError> {
        let nodes = self.network.nodes().map(|n| n.id).collect::<HashSet<_>>();
        for client @ VClient { id, .. } in &self.clients {
            let vnodes = client.nodes().iter().copied().collect::<HashSet<_>>();
            // CORRECTNESS: Every flow must have a valid source and destination.
            for &VFlow { id, src, dst, .. } in client.flows() {
                if !vnodes.contains(&src) {
                    return Err(SpecError::InvalidFlowSrc { flow: id, src });
                }
                if !vnodes.contains(&dst) {
                    return Err(SpecError::InvalidFlowDst { flow: id, dst });
                }
            }
            match self.mappings.get(&id) {
                Some(map) => {
                    // CORRECTNESS: Every mapping must be from a valid virtual node to a valid
                    // physical node.
                    for (&vnode, &node) in map.iter() {
                        if !vnodes.contains(&vnode) {
                            return Err(SpecError::InvalidMappingFrom {
                                client: *id,
                                from: vnode,
                            });
                        }
                        if !nodes.contains(&node) {
                            return Err(SpecError::InvalidMappingTo {
                                client: *id,
                                to: node,
                            });
                        }
                    }
                }
                // CORRECTNESS: Every client must have an entry in `ClientMap`.
                None => return Err(SpecError::MissingClientMapping(*id)),
            }
        }
        Ok(ValidSpec {
            network: self.network,
            clients: self.clients,
            mappings: self.mappings,
        })
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
    InvalidMappingTo { client: ClientId, to: NodeId },
}
