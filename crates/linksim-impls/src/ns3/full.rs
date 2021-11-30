use std::{
    collections::HashMap,
    fmt::Write,
    fs, io,
    path::{Path, PathBuf},
};

use parsimon_core::{
    linksim::{LinkSim, LinkSimError, LinkSimResult},
    network::{EdgeIndex, NodeKind, SimNetwork},
};

#[derive(Debug)]
pub struct Ns3Full {
    root: PathBuf,
}

impl Ns3Full {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            root: PathBuf::from(dir.as_ref()),
        }
    }

    fn write_config(&self, file: impl AsRef<Path>, contents: &str) -> io::Result<()> {
        let path = [self.root.as_path(), file.as_ref()]
            .into_iter()
            .collect::<PathBuf>();
        fs::write(path, contents)
    }

    // TODO
    // fn run_ns3(&self) ->
}

impl LinkSim for Ns3Full {
    fn simulate(&self, network: &SimNetwork, edge: EdgeIndex) -> LinkSimResult {
        fs::create_dir_all(&self.root)?;
        let topology = ns3_topology(network);
        self.write_config("topology.txt", &topology)?;
        let flows = ns3_flows(&network, edge)?;
        self.write_config("flows.txt", &flows)?;
        todo!()
    }
}

/// Get a string representation of the network topology for input to ns-3.
/// TODO: All links that aren't the target link need to be huge
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
fn ns3_flows(network: &SimNetwork, edge: EdgeIndex) -> Result<String, LinkSimError> {
    let flow_map = network
        .flows()
        .map(|f| (f.id, f))
        .collect::<HashMap<_, _>>();
    let chan = network.edge(edge).ok_or(LinkSimError::UnknownEdge(edge))?;
    let nr_flows = chan.nr_flows();
    // First line: # of flows
    // src0 dst0 3 dst_port0 size0 start_time0
    // src1 dst1 3 dst_port1 size1 start_time1
    let lines = std::iter::once(nr_flows.to_string())
        .chain(chan.flows().map(|id| {
            let f = *flow_map.get(&id).unwrap();
            format!(
                "{} {} 3 100 {} {}",
                f.src,
                f.dst,
                f.size.into_u64(),
                f.start.into_u64()
            )
        }))
        .collect::<Vec<_>>();
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parsimon_core::{
        client::ClientId,
        network::{Flow, FlowId, Network, NodeId, UniqFlowId},
        testing,
        units::{Bytes, Nanosecs},
    };

    fn test_sim_network() -> anyhow::Result<SimNetwork> {
        let (nodes, links) = testing::eight_node_config();
        let network = Network::new(&nodes, &links)?;
        let flows = vec![
            Flow {
                id: UniqFlowId::new(ClientId::new(0), FlowId::new(0)),
                src: NodeId::new(0),
                dst: NodeId::new(1),
                size: Bytes::new(1234),
                start: Nanosecs::new(1_000_000_000),
            },
            Flow {
                id: UniqFlowId::new(ClientId::new(0), FlowId::new(1)),
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

    #[test]
    fn ns3_flows_correct() -> anyhow::Result<()> {
        let network = test_sim_network()?;
        let s = ns3_flows(&network, EdgeIndex::new(0))?;
        insta::assert_snapshot!(s, @r###"
        2
        0 1 3 100 1234 1000000000
        0 2 3 100 5678 2000000000
        "###);
        Ok(())
    }
}
