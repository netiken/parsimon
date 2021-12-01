use std::collections::HashSet;

use crate::{
    client::Client,
    linksim::LinkSim,
    network::{types::NodeId, Flow, Network, UniqFlowId},
};

#[derive(Debug, typed_builder::TypedBuilder)]
pub struct Spec<S> {
    network: Network,
    clients: Vec<Client>,
    linksim: S,
}

impl<S: LinkSim> Spec<S> {
    /// Validate a specification, producing a `ValidSpec`.
    ///
    /// Correctness properties:
    ///
    /// - Every flow must have a valid source and destination.
    pub(crate) fn validate(self) -> Result<ValidSpec<S>, SpecError> {
        let nodes = self.network.nodes().map(|n| n.id).collect::<HashSet<_>>();
        for client in &self.clients {
            // CORRECTNESS: Every flow must have a valid source and destination.
            for &Flow { id, src, dst, .. } in client.flows() {
                if !nodes.contains(&src) {
                    return Err(SpecError::InvalidFlowSrc { flow: id, src });
                }
                if !nodes.contains(&dst) {
                    return Err(SpecError::InvalidFlowDst { flow: id, dst });
                }
            }
        }
        Ok(ValidSpec {
            network: self.network,
            clients: self.clients,
            linksim: self.linksim,
        })
    }
}

/// A `ValidSpec` is exactly the same thing as a `Spec`, except it can only be
/// created through a call to `Spec::validate`, and it has public fields.
#[derive(Debug)]
pub(crate) struct ValidSpec<S> {
    pub(crate) network: Network,
    pub(crate) clients: Vec<Client>,
    pub(crate) linksim: S,
}

impl<S: LinkSim> ValidSpec<S> {
    /// Collect all the flows in the specification. The virtual flows are
    /// translated to physical flows, but they are unsorted.
    pub(crate) fn collect_flows(&self) -> Vec<Flow> {
        self.clients
            .iter()
            .flat_map(|c| c.flows().iter().cloned())
            .collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SpecError {
    #[error("flow {flow} has an invalid source ({src})")]
    InvalidFlowSrc { flow: UniqFlowId, src: NodeId },

    #[error("flow {flow} has an invalid source ({dst})")]
    InvalidFlowDst { flow: UniqFlowId, dst: NodeId },
}

#[cfg(test)]
mod tests {
    use petgraph::graph::EdgeIndex;

    use crate::client::ClientId;
    use crate::linksim::LinkSimResult;
    use crate::network::{FlowId, SimNetwork};
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
        let flow = Flow {
            id: UniqFlowId::new(ClientId::new(0), FlowId::new(1)),
            src: NodeId::new(100),
            dst: NodeId::new(2),
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
        let flow = Flow {
            id: UniqFlowId::new(ClientId::new(0), FlowId::new(1)),
            src: NodeId::new(0),
            dst: NodeId::new(100),
            size: Bytes::ZERO,
            start: Nanosecs::ZERO,
        };
        spec.clients[0].flows_mut().push(flow);
        assert!(matches!(
            spec.validate(),
            Err(SpecError::InvalidFlowDst { .. })
        ));
    }

    struct TestLinkSim;

    impl LinkSim for TestLinkSim {
        fn simulate(&self, _: &SimNetwork, _: EdgeIndex) -> LinkSimResult {
            unreachable!()
        }
    }

    fn spec() -> Spec<TestLinkSim> {
        let network = network();
        let client = client();
        Spec {
            network,
            clients: vec![client],
            linksim: TestLinkSim,
        }
    }

    fn network() -> Network {
        let (nodes, links) = testing::eight_node_config();
        Network::new(&nodes, &links).unwrap()
    }

    fn client() -> Client {
        let flow = Flow {
            id: UniqFlowId::new(ClientId::new(0), FlowId::new(0)),
            src: NodeId::new(0),
            dst: NodeId::new(2),
            size: Bytes::ZERO,
            start: Nanosecs::ZERO,
        };
        Client::new(ClientId::ZERO, "test-client".into(), vec![flow])
    }
}
