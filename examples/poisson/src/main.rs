use clap::Parser;
use parsimon::{
    core::{
        cluster::DefaultClustering,
        network::{
            types::{Flow, FlowId, Link, Node, NodeId},
            DelayNetwork,
        },
        opts::SimOpts,
        run::run,
        spec::Spec,
        units::{BitsPerSec, Bytes, Gbps, Mbps, Nanosecs},
    },
    impls::linksim::MinimLink,
};
use rand::prelude::*;
use rand_distr::Exp;

const WINDOW: Bytes = Bytes::new(18_000);
const DCTCP_GAIN: f64 = 0.0625;
const DCTCP_AI: Mbps = Mbps::new(615);

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Average flow size
    #[arg(short, long, default_value = "10000")]
    flow: Bytes,

    /// Average Load level
    #[arg(short, long, default_value_t = 0.2)]
    load: f64,

    /// Number of flows
    #[arg(short, long, default_value_t = 100_000)]
    nr_flows: usize,

    /// Random seed
    #[arg(short, long, default_value_t = 0)]
    seed: u64,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    anyhow::ensure!(
        0.0 < args.load && args.load < 1.0,
        "load must be between 0.0 and 1.0, exclusive"
    );

    let mut rng = StdRng::seed_from_u64(args.seed);
    let (nodes, links) = eight_node_config();
    let flows = gen_flows(args.flow, args.load, args.nr_flows, &mut rng)?;
    let spec = Spec::builder()
        .nodes(nodes)
        .links(links)
        .flows(flows.clone())
        .build();
    let minim = MinimLink::builder()
        .window(WINDOW)
        .dctcp_gain(DCTCP_GAIN)
        .dctcp_ai(DCTCP_AI)
        .build();
    let opts = SimOpts::builder().link_sim(minim).build();

    let delay_network: DelayNetwork = run(spec, opts, DefaultClustering)?;

    // feed all flows back into delay network
    let mut ns_predictions = flows
        .iter()
        .map(|flow| {
            delay_network
                .predict(flow.size, (flow.src, flow.dst), &mut rng)
                .ok_or_else(|| anyhow::anyhow!("failed to get prediction"))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    ns_predictions.sort();

    // print percentiles
    let p50_idx = (0.50 * ns_predictions.len() as f64) as usize;
    let p95_idx = (0.95 * ns_predictions.len() as f64) as usize;
    let p99_idx = (0.99 * ns_predictions.len() as f64) as usize;
    println!("The 50th percentile is: {:?}", ns_predictions[p50_idx]);
    println!("The 95th percentile is: {:?}", ns_predictions[p95_idx]);
    println!("The 99th percentile is: {:?}", ns_predictions[p99_idx]);
    Ok(())
}

/// Generate a configuration with four hosts (IDs 0-3), two ToR switches (IDs 4-5), and two agg
/// switches (IDs 6-7) organized in a Clos topology. Each ToR is connected to two hosts.
///
/// Links are 10 Gbps with a 1 us propagation delay.
pub fn eight_node_config() -> (Vec<Node>, Vec<Link>) {
    let hosts = (0..=3).map(|i| Node::new_host(NodeId::new(i)));
    let switches = (4..=7).map(|i| Node::new_switch(NodeId::new(i)));
    let nodes = hosts.chain(switches).collect::<Vec<_>>();
    let links = vec![
        Link::new(nodes[0].id, nodes[4].id, Gbps::new(10), Nanosecs::new(1000)),
        Link::new(nodes[1].id, nodes[4].id, Gbps::new(10), Nanosecs::new(1000)),
        Link::new(nodes[2].id, nodes[5].id, Gbps::new(10), Nanosecs::new(1000)),
        Link::new(nodes[3].id, nodes[5].id, Gbps::new(10), Nanosecs::new(1000)),
        // Each ToR is connected to both Aggs
        Link::new(nodes[4].id, nodes[6].id, Gbps::new(10), Nanosecs::new(1000)),
        Link::new(nodes[4].id, nodes[7].id, Gbps::new(10), Nanosecs::new(1000)),
        Link::new(nodes[5].id, nodes[6].id, Gbps::new(10), Nanosecs::new(1000)),
        Link::new(nodes[5].id, nodes[7].id, Gbps::new(10), Nanosecs::new(1000)),
    ];
    (nodes, links)
}

fn gen_flows(
    mean_flow_size: Bytes,
    mean_load: f64,
    nr_flows: usize,
    mut rng: impl Rng,
) -> anyhow::Result<Vec<Flow>> {
    // Calculate mean interarrival time T (ns) for one server
    // Bandwidth (bps) * Load (bps/bps) = desired rate (bps)
    // flow size (bytes) to flow size (bits) / desired rate (bps) --> seconds --> ns
    let bandwidth_bps = BitsPerSec::from(Gbps::new(10));
    let desired_rate = bandwidth_bps.into_f64() * mean_load;
    let mean_interarrival_time =
        Nanosecs::new(((mean_flow_size.into_f64() * 8.0 * 1e9) / (desired_rate * 4.0)) as u64);

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
            size: Bytes::new(flow_exp.sample(&mut rng).round() as u64),
            start: Nanosecs::new(new_start),
        });
        prev_start = new_start;
    }
    Ok(flows)
}
