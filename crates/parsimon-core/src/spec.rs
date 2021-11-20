use std::collections::HashSet;

use crate::{
    client::{ClientId, ClientMap, VClient, VFlow, VNodeId},
    linksim::LinkSim,
    network::{types::NodeId, Flow, Network, UniqFlowId},
};

#[derive(Debug, typed_builder::TypedBuilder)]
pub struct Spec<S> {
    network: Network,
    clients: Vec<VClient>,
    mappings: ClientMap,
    linksim: S,
}

impl<S: LinkSim> Spec<S> {
    /// Validate a specification, producing a `ValidSpec`.
    ///
    /// Correctness properties:
    ///
    /// - Every flow must have a valid source and destination.
    /// - Every client must have an entry in `ClientMap`.
    /// - Every mapping must be from a valid virtual node to a valid physical node.
    pub(crate) fn validate(self) -> Result<ValidSpec<S>, SpecError> {
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
            linksim: self.linksim,
        })
    }
}

/// A `ValidSpec` is exactly the same thing as a `Spec`, except it can only be
/// created through a call to `Spec::validate`, and it has public fields.
#[derive(Debug)]
pub(crate) struct ValidSpec<S> {
    pub(crate) network: Network,
    pub(crate) clients: Vec<VClient>,
    pub(crate) mappings: ClientMap,
    pub(crate) linksim: S,
}

impl<S: LinkSim> ValidSpec<S> {
    /// Collect all the flows in the specification. The virtual flows are
    /// translated to physical flows, but they are unsorted.
    pub(crate) fn collect_flows(&self) -> Vec<Flow> {
        self.clients
            .iter()
            .flat_map(|c| {
                c.flows().iter().map(|vf| {
                    // The `unwrap`s are justified by the correctness condition in
                    // `Spec::validate`.
                    let map = self.mappings.get(&c.id).unwrap();
                    Flow {
                        id: vf.id,
                        src: *map.get(&vf.src).unwrap(),
                        dst: *map.get(&vf.dst).unwrap(),
                        size: vf.size,
                        start: vf.start,
                    }
                })
            })
            .collect()
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

#[cfg(test)]
mod tests {
    use petgraph::graph::EdgeIndex;

    use crate::edist::EDistBuckets;
    use crate::network::{FctRecord, FlowId, SimNetwork};
    use crate::testing;
    use crate::units::{Bytes, Nanosecs};

    use super::*;

    #[test]
    fn valid_spec_succeeds() {
        let spec = spec();
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn invalid_flow_src_fails() {
        let mut spec = spec();
        let flow = VFlow {
            id: UniqFlowId::new(ClientId::new(0), FlowId::new(1)),
            src: VNodeId::new(100),
            dst: VNodeId::new(2),
            size: Bytes::ZERO,
            start: Nanosecs::ZERO,
        };
        spec.clients[0].flows_mut().push(flow);
        assert!(matches!(
            spec.validate(),
            Err(SpecError::InvalidFlowSrc { .. })
        ));
    }

    #[test]
    fn invalid_flow_dst_fails() {
        let mut spec = spec();
        let flow = VFlow {
            id: UniqFlowId::new(ClientId::new(0), FlowId::new(1)),
            src: VNodeId::new(0),
            dst: VNodeId::new(100),
            size: Bytes::ZERO,
            start: Nanosecs::ZERO,
        };
        spec.clients[0].flows_mut().push(flow);
        assert!(matches!(
            spec.validate(),
            Err(SpecError::InvalidFlowDst { .. })
        ));
    }

    #[test]
    fn missing_client_mapping_fails() {
        let mut spec = spec();
        spec.mappings.remove(&ClientId::new(0));
        assert!(matches!(
            spec.validate(),
            Err(SpecError::MissingClientMapping(..))
        ));
    }

    #[test]
    fn invalid_mapping_from_fails() {
        let mut spec = spec();
        let map = spec.mappings.get_mut(&ClientId::new(0)).unwrap();
        map.insert(VNodeId::new(100), NodeId::new(0));
        assert!(matches!(
            spec.validate(),
            Err(SpecError::InvalidMappingFrom { .. })
        ));
    }

    #[test]
    fn invalid_mapping_to_fails() {
        let mut spec = spec();
        let map = spec.mappings.get_mut(&ClientId::new(0)).unwrap();
        map.insert(VNodeId::new(0), NodeId::new(100));
        assert!(matches!(
            spec.validate(),
            Err(SpecError::InvalidMappingTo { .. })
        ));
    }

    struct TestLinkSim;

    impl LinkSim for TestLinkSim {
        fn simulate(&self, _: &SimNetwork, _: EdgeIndex) -> Vec<FctRecord> {
            unreachable!()
        }
    }

    fn spec() -> Spec<TestLinkSim> {
        let network = network();
        let client = client();
        let mappings = mappings();
        Spec {
            network,
            clients: vec![client],
            mappings,
            linksim: TestLinkSim,
        }
    }

    fn network() -> Network {
        let (nodes, links) = testing::eight_node_config();
        Network::new(&nodes, &links).unwrap()
    }

    fn client() -> VClient {
        let flow = VFlow {
            id: UniqFlowId::new(ClientId::new(0), FlowId::new(0)),
            src: VNodeId::new(0),
            dst: VNodeId::new(2),
            size: Bytes::ZERO,
            start: Nanosecs::ZERO,
        };
        VClient::new(
            ClientId::ZERO,
            "test-client".into(),
            vec![VNodeId::new(0), VNodeId::new(1), VNodeId::new(2)],
            vec![flow],
        )
    }

    fn mappings() -> ClientMap {
        [(
            ClientId::new(0),
            [
                (VNodeId::new(0), NodeId::new(0)),
                (VNodeId::new(1), NodeId::new(1)),
                (VNodeId::new(2), NodeId::new(2)),
            ]
            .into_iter()
            .collect(),
        )]
        .into_iter()
        .collect()
    }
}
