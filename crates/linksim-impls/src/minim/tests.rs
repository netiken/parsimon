use std::collections::BTreeMap;

use minim::{
    queue::FifoQ,
    units::{BitsPerSec, Bytes, Nanosecs},
    Config, FlowDesc, SourceDesc,
};
use parsimon_core::{
    linksim::LinkSimSpec,
    network::{Flow, FlowId, Network, NodeId},
    testing,
};
use rand::prelude::*;
use rand_distr::Exp;
use rustc_hash::FxHashMap;

use super::MinimLink;

#[derive(Debug, serde::Serialize)]
struct MinimCheck {
    bandwidth: BitsPerSec,
    sources: Vec<SourceDesc>,
    flows: Vec<FlowDesc>,
    window: Bytes,
    dctcp_marking_threshold: Bytes,
    dctcp_gain: f64,
    dctcp_ai: BitsPerSec,
    sz_pktmax: Bytes,
    sz_pkthdr: Bytes,
    timeout: Option<Nanosecs>,
}

impl MinimCheck {
    fn from_config(cfg: &Config<FifoQ>) -> Self {
        Self {
            bandwidth: cfg.bandwidth,
            sources: cfg.sources.clone(),
            flows: cfg.flows.clone(),
            window: cfg.window,
            dctcp_marking_threshold: cfg.dctcp_marking_threshold,
            dctcp_gain: cfg.dctcp_gain,
            dctcp_ai: cfg.dctcp_ai,
            sz_pktmax: cfg.sz_pktmax,
            sz_pkthdr: cfg.sz_pkthdr,
            timeout: cfg.timeout,
        }
    }
}

type Snapshot = BTreeMap<(NodeId, NodeId), MinimCheck>;

fn eight_node_config_snapshots(flows: Vec<Flow>) -> anyhow::Result<Snapshot> {
    // Build a `Network`.
    let (nodes, links) = testing::eight_node_config();
    let network = Network::new(&nodes, &links)?;

    // Convert the `Network` into a `SimNetwork` by adding flows.
    let id2flow = flows
        .iter()
        .map(|f| (f.id, f.to_owned()))
        .collect::<FxHashMap<_, _>>();
    let network = network.into_simulations(flows);

    // Build a `MinimLink` instance and use it to generate `MinimCheck`s.
    let linksim = MinimLink::builder()
        .window(parsimon_core::units::Bytes::new(18_000))
        .dctcp_gain(0.0625)
        .dctcp_ai(parsimon_core::units::Mbps::new(615))
        .build();
    let snapshot = network
        .edge_indices()
        .filter_map(|eidx| network.link_sim_desc(eidx))
        .map(|desc| {
            let flows = desc
                .flows
                .iter()
                .map(|id| id2flow.get(id).unwrap().to_owned())
                .collect::<Vec<_>>();
            let spec = LinkSimSpec {
                edge: desc.edge,
                bottleneck: desc.bottleneck,
                other_links: desc.other_links,
                nodes: desc.nodes,
                flows,
            };
            let (bsrc, bdst) = (spec.bottleneck.from, spec.bottleneck.to);
            let cfg = linksim.build_config(spec)?;
            let check = MinimCheck::from_config(&cfg);
            Ok(((bsrc, bdst), check))
        })
        .collect::<anyhow::Result<BTreeMap<(NodeId, NodeId), MinimCheck>>>()?;
    Ok(snapshot)
}

fn gen_flows(
    mean_flow_size: parsimon_core::units::Bytes,
    mean_load: f64,
    nr_flows: usize,
    mut rng: impl Rng,
) -> anyhow::Result<Vec<Flow>> {
    // Calculate mean interarrival time T (ns) for one server
    // Bandwidth (bps) * Load (bps/bps) = desired rate (bps)
    // flow size (bytes) to flow size (bits) / desired rate (bps) --> seconds --> ns
    let bandwidth_bps = parsimon_core::units::BitsPerSec::from(parsimon_core::units::Gbps::new(10));
    let desired_rate = bandwidth_bps.into_f64() * mean_load;
    let mean_interarrival_time = parsimon_core::units::Nanosecs::new(
        ((mean_flow_size.into_f64() * 8.0 * 1e9) / (desired_rate * 4.0)) as u64,
    );

    // Make exponential distributions
    // Flow size distribution
    let flow_exp = Exp::new(mean_flow_size.into_f64().recip())?;
    let start_exp = Exp::new(mean_interarrival_time.into_f64().recip())?;
    let mut node_nums: Vec<usize> = (0..4).collect();

    // Draw flows from distribution
    let mut flows: Vec<Flow> = Vec::new();
    let mut prev_start: u64 = 0;
    for i in 0..nr_flows {
        node_nums.shuffle(&mut rng);
        let new_start: u64 = start_exp.sample(&mut rng).round() as u64 + prev_start;
        flows.push(Flow {
            id: FlowId::new(i),
            src: NodeId::new(node_nums[0]),
            dst: NodeId::new(node_nums[1]),
            size: parsimon_core::units::Bytes::new(flow_exp.sample(&mut rng).round() as u64),
            start: parsimon_core::units::Nanosecs::new(new_start),
        });
        prev_start = new_start;
    }
    Ok(flows)
}

#[test]
fn config_correct() -> anyhow::Result<()> {
    let snapshot = eight_node_config_snapshots(vec![
        Flow {
            id: FlowId::ZERO,
            src: NodeId::new(0),
            dst: NodeId::new(2),
            size: parsimon_core::units::Bytes::new(1000),
            start: parsimon_core::units::Nanosecs::ZERO,
        },
        Flow {
            id: FlowId::ONE,
            src: NodeId::new(0),
            dst: NodeId::new(2),
            size: parsimon_core::units::Bytes::new(1000),
            start: parsimon_core::units::Nanosecs::new(960),
        },
    ])?;
    insta::assert_yaml_snapshot!(snapshot);
    Ok(())
}

#[test]
fn config_correct_loaded() -> anyhow::Result<()> {
    let rng = StdRng::seed_from_u64(0);
    let flows = gen_flows(parsimon_core::units::Bytes::new(10_000), 0.5, 10000, rng)?;
    let snapshot = eight_node_config_snapshots(flows)?;
    insta::assert_yaml_snapshot!(snapshot);
    Ok(())
}
