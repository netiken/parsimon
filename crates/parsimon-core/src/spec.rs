//! This module defines simulation specifications ([`Spec`]), which consists of nodes, links, and
//! flows. `Parsimon` turns a specification into a [`DelayNetwork`](crate::network::DelayNetwork),
//! which can be queried for FCT delay estimates.

use std::collections::HashSet;

use crate::{
    network::{
        types::{Link, Node, NodeId},
        Flow, FlowId, Network, NodeKind, TopologyError,
    },
};

/// A simulation specification.
#[derive(Debug, typed_builder::TypedBuilder)]
pub struct Spec {
    /// Topology nodes.
    pub nodes: Vec<Node>,
    /// Topology links.
    pub links: Vec<Link>,
    /// Workload flows.
    pub flows: Vec<Flow>,
}

impl Spec {
    /// Validate a specification, producing a `ValidSpec`.
    ///
    /// Correctness properties:
    ///
    /// - Every flow must have a valid source and destination
    // TODO: Flow IDs should be unique
    pub(crate) fn validate(self) -> Result<ValidSpec, SpecError> {
        let hosts = self
            .nodes
            .iter()
            .filter_map(|n| match n.kind {
                NodeKind::Host => Some(n.id),
                NodeKind::Switch => None,
            })
            .collect::<HashSet<_>>();
        // CORRECTNESS: Every flow must have a valid source and destination.
        for &Flow { id, src, dst, .. } in &self.flows {
            if !hosts.contains(&src) {
                return Err(SpecError::InvalidFlowSrc { flow: id, src });
            }
            if !hosts.contains(&dst) {
                return Err(SpecError::InvalidFlowDst { flow: id, dst });
            }
        }
        let network = Network::new(&self.nodes, &self.links)?;
        Ok(ValidSpec {
            network,
            flows: self.flows,
        })
    }
}

/// A `ValidSpec` is a `Spec` that has been validated. The topology and the
/// flows are guaranteed to satisfy properties listed in `Network::new()` and
/// `Spec::validate()`.
#[derive(Debug)]
pub(crate) struct ValidSpec {
    pub(crate) network: Network,
    pub(crate) flows: Vec<Flow>,
}

impl ValidSpec {
    pub(crate) fn collect_flows(&self) -> Vec<Flow> {
        self.flows.to_vec()
    }
}

/// Simulation specification error.
#[derive(Debug, thiserror::Error)]
pub enum SpecError {
    /// A flow has an invalid source.
    #[error("flow {flow} has an invalid source ({src})")]
    InvalidFlowSrc {
        /// The flow ID.
        flow: FlowId,
        /// The invalid source.
        src: NodeId,
    },

    /// A flow has an invalid destination.
    #[error("flow {flow} has an invalid source ({dst})")]
    InvalidFlowDst {
        /// The flow ID.
        flow: FlowId,
        /// The invalid destination.
        dst: NodeId,
    },

    /// The topology is invalid.
    #[error("invalid topology")]
    InvalidTopology(#[from] TopologyError),
}

#[cfg(test)]
mod tests {
    use crate::network::FlowId;
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

    fn spec() -> Spec {
        let (nodes, links) = testing::eight_node_config();
        let flows = flows();
        Spec {
            nodes,
            links,
            flows,
        }
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
