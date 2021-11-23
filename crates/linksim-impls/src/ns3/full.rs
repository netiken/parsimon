use std::fmt::Write;

use parsimon_core::{
    linksim::LinkSim,
    network::{EdgeIndex, FctRecord, NodeKind, SimNetwork},
};

#[derive(Debug)]
pub struct Ns3Full;

impl LinkSim for Ns3Full {
    fn simulate(&self, network: &SimNetwork, edge: EdgeIndex) -> Vec<FctRecord> {
        todo!()
    }
}

/// Get a string representation of the network topology for input to ns-3.
fn ns3_topology(network: &SimNetwork) -> String {
    let nodes = network.nodes().collect::<Vec<_>>();
    let switches = nodes
        .iter()
        .filter(|&&n| matches!(n.kind, NodeKind::Switch))
        .collect::<Vec<_>>();
    let links = network.links().collect::<Vec<_>>();
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

/// Get a string representation of the network flows for input to ns-3.
/// TODO: Write me next
fn ns3_flows(network: &SimNetwork) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use parsimon_core::{network::Network, testing};

    fn sim_network() -> anyhow::Result<SimNetwork> {
        let (nodes, links) = testing::eight_node_config();
        let network = Network::new(&nodes, &links)?;
        let network = network.into_simulations(vec![]);
        Ok(network)
    }

    #[test]
    fn ns3_topology_correct() -> anyhow::Result<()> {
        let network = sim_network()?;
        let s = ns3_topology(&network);
        insta::assert_snapshot!(s, @r###"
        8 4 8
        4 5 6 7
        0 4 0Gbps 0ns 0
        1 4 0Gbps 0ns 0
        2 5 0Gbps 0ns 0
        3 5 0Gbps 0ns 0
        4 6 0Gbps 0ns 0
        4 7 0Gbps 0ns 0
        5 6 0Gbps 0ns 0
        5 7 0Gbps 0ns 0
        "###);
        Ok(())
    }
}
