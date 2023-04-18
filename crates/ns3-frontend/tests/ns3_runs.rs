use ns3_frontend::Ns3Simulation;
use parsimon_core::{
    network::{Flow, FlowId, NodeId},
    units::{Bytes, Nanosecs},
};

#[test]
#[ignore = "ns-3 needs to be compiled"]
fn ns3_runs() -> anyhow::Result<()> {
    const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
    let data_dir = tempfile::tempdir()?;
    let ns3_dir =
        format!("{MANIFEST_DIR}/../../backends/High-Precision-Congestion-Control/simulation",);
    let (nodes, links) = parsimon_core::testing::eight_node_config();
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
    let sim = Ns3Simulation::builder()
        .ns3_dir(ns3_dir)
        .data_dir(data_dir.path())
        .nodes(nodes)
        .links(links)
        .window(Bytes::new(100_000))
        .base_rtt(Nanosecs::new(8_000))
        .flows(flows)
        .build();
    let records = sim.run()?;
    assert_eq!(records.len(), 2);
    Ok(())
}
