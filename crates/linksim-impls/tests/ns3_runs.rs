use linksim_impls::ns3::full::Ns3Full;
use parsimon_core::{
    client::ClientId,
    linksim::LinkSim,
    network::{EdgeIndex, Flow, FlowId, Network, NodeId, SimNetwork, UniqFlowId},
    testing,
    units::{Bytes, Nanosecs},
};

#[test]
fn ns3_runs() -> anyhow::Result<()> {
    const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
    let root_dir = tempfile::tempdir()?;
    let ns3_dir = format!(
        "{}/../../backends/High-Precision-Congestion-Control/simulation",
        MANIFEST_DIR
    );
    let sim = Ns3Full::new(root_dir.path(), ns3_dir);
    let network = test_sim_network()?;
    assert_eq!(sim.simulate(&network, EdgeIndex::new(0))?.len(), 2);
    Ok(())
}

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
