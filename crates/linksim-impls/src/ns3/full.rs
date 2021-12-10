use std::{
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use parsimon_core::{
    linksim::{LinkSim, LinkSimError, LinkSimResult},
    network::{
        types::{Link, TracedChannel},
        EdgeIndex, NodeKind, SimNetwork,
    },
    units::Bytes,
};

use super::Ns3Sim;

#[derive(Debug)]
pub struct Ns3Full {
    root_dir: PathBuf,
    ns3_dir: PathBuf,
    window: Bytes,
}

impl Ns3Full {
    pub fn new(root_dir: impl AsRef<Path>, ns3_dir: impl AsRef<Path>, window: Bytes) -> Self {
        Self {
            root_dir: PathBuf::from(root_dir.as_ref()),
            ns3_dir: PathBuf::from(ns3_dir.as_ref()),
            window,
        }
    }
}

impl Ns3Sim for Ns3Full {
    fn root_dir(&self) -> &Path {
        self.root_dir.as_path()
    }

    fn ns3_dir(&self) -> &Path {
        self.ns3_dir.as_path()
    }

    fn window(&self) -> Bytes {
        self.window
    }

    fn to_ns3_topology(network: &SimNetwork, chan: &TracedChannel) -> String {
        let nodes = network.nodes().collect::<Vec<_>>();
        let switches = nodes
            .iter()
            .filter(|&&n| matches!(n.kind, NodeKind::Switch))
            .collect::<Vec<_>>();
        let links = network
            .links()
            .map(|l| {
                if l.connects(chan.src(), chan.dst()) {
                    l.to_owned()
                } else {
                    // Scale the link capacity by a factor of 10
                    Link {
                        bandwidth: l.bandwidth.scale_by(10.0),
                        ..*l
                    }
                }
            })
            .collect::<Vec<_>>();
        let mut s = String::new();
        // First line: total node #, switch node #, link #
        writeln!(s, "{} {} {}", nodes.len(), switches.len(), links.len()).unwrap();
        // Second line: switch node IDs...
        let switch_ids = switches
            .iter()
            .map(|&&s| s.id.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        writeln!(s, "{}", switch_ids).unwrap();
        // src0 dst0 rate delay error_rate
        // src1 dst1 rate delay error_rate
        // ...
        for link in links {
            writeln!(
                s,
                "{} {} {} {} 0",
                link.a, link.b, link.bandwidth, link.delay
            )
            .unwrap();
        }
        s
    }
}

linksim_impl!(Ns3Full);

#[cfg(test)]
mod tests {
    use super::*;

    use parsimon_core::{
        network::{Flow, FlowId, Network, NodeId},
        testing,
        units::{Bytes, Nanosecs},
    };

    fn test_sim_network() -> anyhow::Result<SimNetwork> {
        let (nodes, links) = testing::eight_node_config();
        let network = Network::new(&nodes, &links)?;
        let flows = vec![
            Flow {
                id: FlowId::new(0),
                src: NodeId::new(0),
                dst: NodeId::new(1),
                size: Bytes::new(1234),
                start: Nanosecs::new(1_000_000_000),
            },
            Flow {
                id: FlowId::new(1),
                src: NodeId::new(0),
                dst: NodeId::new(2),
                size: Bytes::new(5678),
                start: Nanosecs::new(2_000_000_000),
            },
        ];
        let network = network.into_simulations(flows);
        Ok(network)
    }

    #[test]
    fn ns3_topology_correct() -> anyhow::Result<()> {
        let network = test_sim_network()?;
        let edge = EdgeIndex::new(0);
        let chan = network.edge(edge).ok_or(LinkSimError::UnknownEdge(edge))?;
        let s = Ns3Full::to_ns3_topology(&network, chan);
        insta::assert_snapshot!(s, @r###"
        8 4 8
        4 5 6 7
        0 4 100Gbps 1000ns 0
        1 4 1000Gbps 1000ns 0
        2 5 1000Gbps 1000ns 0
        3 5 1000Gbps 1000ns 0
        4 6 1000Gbps 1000ns 0
        4 7 1000Gbps 1000ns 0
        5 6 1000Gbps 1000ns 0
        5 7 1000Gbps 1000ns 0
        "###);
        Ok(())
    }

    #[test]
    fn ns3_flows_correct() -> anyhow::Result<()> {
        let network = test_sim_network()?;
        let edge = EdgeIndex::new(0);
        let chan = network.edge(edge).ok_or(LinkSimError::UnknownEdge(edge))?;
        let s = Ns3Full::to_ns3_flows(&network, chan)?;
        insta::assert_snapshot!(s, @r###"
        2
        0 0 1 3 100 1234 1
        1 0 2 3 100 5678 2
        "###);
        Ok(())
    }
}
