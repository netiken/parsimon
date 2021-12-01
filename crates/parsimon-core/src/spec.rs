use std::collections::HashSet;

use crate::{
    linksim::LinkSim,
    network::{types::NodeId, Flow, FlowId, Network},
};

#[derive(Debug, typed_builder::TypedBuilder)]
pub struct Spec<S> {
    network: Network,
    flows: Vec<Flow>,
    linksim: S,
}

impl<S: LinkSim> Spec<S> {
    /// Validate a specification, producing a `ValidSpec`.
    ///
    /// Correctness properties:
    ///
    /// - Every flow must have a valid source and destination
    // TODO: Flow IDs should be unique
    pub(crate) fn validate(self) -> Result<ValidSpec<S>, SpecError> {
        // FIXME: This should only contain the set of hosts
        let nodes = self.network.nodes().map(|n| n.id).collect::<HashSet<_>>();
        // CORRECTNESS: Every flow must have a valid source and destination.
        for &Flow { id, src, dst, .. } in &self.flows {
            if !nodes.contains(&src) {
                return Err(SpecError::InvalidFlowSrc { flow: id, src });
            }
            if !nodes.contains(&dst) {
                return Err(SpecError::InvalidFlowDst { flow: id, dst });
            }
        }
        Ok(ValidSpec {
            network: self.network,
            flows: self.flows,
            linksim: self.linksim,
        })
    }
}

/// A `ValidSpec` is exactly the same thing as a `Spec`, except it can only be
/// created through a call to `Spec::validate`, and it has public fields.
#[derive(Debug)]
pub(crate) struct ValidSpec<S> {
    pub(crate) network: Network,
    pub(crate) flows: Vec<Flow>,
    pub(crate) linksim: S,
}

impl<S: LinkSim> ValidSpec<S> {
    pub(crate) fn collect_flows(&self) -> Vec<Flow> {
        self.flows.iter().cloned().collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SpecError {
    #[error("flow {flow} has an invalid source ({src})")]
    InvalidFlowSrc { flow: FlowId, src: NodeId },

    #[error("flow {flow} has an invalid source ({dst})")]
    InvalidFlowDst { flow: FlowId, dst: NodeId },
}

#[cfg(test)]
mod tests {
    use petgraph::graph::EdgeIndex;

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
            id: FlowId::new(1),
            src: NodeId::new(100),
            dst: NodeId::new(2),
            size: Bytes::ZERO,
            start: Nanosecs::ZERO,
        };
        spec.flows.push(flow);
        assert!(matches!(
            spec.validate(),
            Err(SpecError::InvalidFlowSrc { .. })
        ));
    }

    #[test]
    fn invalid_flow_dst_fails() {
        let mut spec = spec();
        let flow = Flow {
            id: FlowId::new(1),
            src: NodeId::new(0),
            dst: NodeId::new(100),
            size: Bytes::ZERO,
            start: Nanosecs::ZERO,
        };
        spec.flows.push(flow);
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
        let flows = flows();
        Spec {
            network,
            flows,
            linksim: TestLinkSim,
        }
    }

    fn network() -> Network {
        let (nodes, links) = testing::eight_node_config();
        Network::new(&nodes, &links).unwrap()
    }

    fn flows() -> Vec<Flow> {
        let flow = Flow {
            id: FlowId::new(0),
            src: NodeId::new(0),
            dst: NodeId::new(2),
            size: Bytes::ZERO,
            start: Nanosecs::ZERO,
        };
        vec![flow]
    }
}
