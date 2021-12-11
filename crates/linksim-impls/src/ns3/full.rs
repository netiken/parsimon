use std::path::{Path, PathBuf};

use ns3_frontend::Ns3Simulation;
use parsimon_core::{
    linksim::{LinkSim, LinkSimError, LinkSimResult},
    network::{types::Link, EdgeIndex, SimNetwork},
    units::Bytes,
};

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

impl LinkSim for Ns3Full {
    fn simulate(&self, network: &SimNetwork, edge: EdgeIndex) -> LinkSimResult {
        let nodes = network.nodes().cloned().collect::<Vec<_>>();
        let chan = network.edge(edge).ok_or(LinkSimError::UnknownEdge(edge))?;
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
        let flows = network.flows_on(edge).unwrap(); // we already know the channel exists
        let mut data_dir = PathBuf::from(&self.root_dir);
        data_dir.push(format!("{}-{}", chan.src(), chan.dst()));
        let sim = Ns3Simulation::builder()
            .ns3_dir(&self.ns3_dir)
            .data_dir(data_dir)
            .nodes(nodes)
            .links(links)
            .window(self.window)
            .flows(flows)
            .build();
        let records = sim.run().map_err(|e| anyhow::anyhow!(e))?;
        Ok(records)
    }
}
