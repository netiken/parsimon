use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use ns3_frontend::Ns3Simulation;
use parsimon_core::{
    linksim::{LinkSim, LinkSimError, LinkSimResult},
    network::{
        types::{Link, Node},
        Channel, EdgeIndex, NodeId, SimNetwork,
    },
    units::{Bytes, Nanosecs},
};

#[derive(Debug)]
pub struct Ns3Link {
    root_dir: PathBuf,
    ns3_dir: PathBuf,
    window: Bytes,
    base_rtt: Nanosecs,
}

impl Ns3Link {
    pub fn new(
        root_dir: impl AsRef<Path>,
        ns3_dir: impl AsRef<Path>,
        window: Bytes,
        base_rtt: Nanosecs,
    ) -> Self {
        Self {
            root_dir: PathBuf::from(root_dir.as_ref()),
            ns3_dir: PathBuf::from(ns3_dir.as_ref()),
            window,
            base_rtt,
        }
    }
}

impl LinkSim for Ns3Link {
    fn simulate(&self, network: &SimNetwork, edge: EdgeIndex) -> LinkSimResult {
        let chan = network.edge(edge).ok_or(LinkSimError::UnknownEdge(edge))?;
        let mut flows = network.flows_on(edge).unwrap(); // we already know the channel exists

        // NOTE: `bsrc` and `bdst` may be in `srcs` and `dsts`, respectfully
        let (srcs, dsts): (HashSet<_>, HashSet<_>) = flows.iter().map(|f| (f.src, f.dst)).unzip();
        let (bsrc, bdst) = (chan.src(), chan.dst());

        assert!(srcs.intersection(&dsts).count() == 0);
        let nodes = srcs
            .iter()
            .chain(dsts.iter())
            .chain([&bsrc, &bdst].into_iter())
            .collect::<HashSet<_>>();
        let mut nodes = nodes
            .into_iter()
            .map(|&id| network.node(id).unwrap())
            .cloned()
            .collect::<Vec<Node>>();

        let mut links = Vec::new();
        // Connect sources to the bottleneck with fat links. If `bsrc` is in `srcs`, then the
        // bottleneck channel is assumed to be a host-ToR up-channel.
        if srcs.contains(&bsrc) {
            assert!(srcs.len() == 1);
        } else {
            for src in srcs {
                // CORRECTNESS: assumes all paths from `src` to `bsrc` have the same min bandwidth
                // and delay
                let path = network.path(src, bsrc, |choices| choices.first());
                let bandwidth = path.bandwidths().min().unwrap().scale_by(10.0);
                let delay = path.delay();
                let link = Link::new(src, bsrc, bandwidth, delay);
                links.push(link);
            }
        }
        // Connect the bottleneck to destinations with fat links. If `bdst` is in `dsts`, then the
        // bottleneck channel is assumed to be a ToR-host down-channel.
        if dsts.contains(&bdst) {
            assert!(dsts.len() == 1);
        } else {
            for dst in dsts {
                // CORRECTNESS: assumes all paths from `bdst` to `dst` have the same min bandwidth
                // and delay
                let path = network.path(bdst, dst, |choices| choices.first());
                let bandwidth = path.bandwidths().min().unwrap().scale_by(10.0);
                let delay = path.delay();
                let link = Link::new(bdst, dst, bandwidth, delay);
                links.push(link);
            }
        }
        // Now include the bottleneck channel
        let bottleneck = Link::new(bsrc, bdst, chan.bandwidth(), chan.delay());
        links.push(bottleneck);

        // The last step is to re-assign node IDs so that they're contiguous.
        let old2new = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id, NodeId::new(i)))
            .collect::<HashMap<_, _>>();
        for node in nodes.iter_mut() {
            node.id = *old2new.get(&node.id).unwrap();
        }
        for link in links.iter_mut() {
            link.a = *old2new.get(&link.a).unwrap();
            link.b = *old2new.get(&link.b).unwrap();
        }
        for flow in flows.iter_mut() {
            flow.src = *old2new.get(&flow.src).unwrap();
            flow.dst = *old2new.get(&flow.dst).unwrap();
        }

        // Set up and run simulation
        let mut data_dir = PathBuf::from(&self.root_dir);
        data_dir.push(format!("{}-{}", chan.src(), chan.dst()));
        let sim = Ns3Simulation::builder()
            .ns3_dir(&self.ns3_dir)
            .data_dir(data_dir)
            .nodes(nodes)
            .links(links)
            .window(self.window)
            .base_rtt(self.base_rtt)
            .flows(flows)
            .build();
        let records = sim.run().map_err(|e| anyhow::anyhow!(e))?;
        Ok(records)
    }
}
