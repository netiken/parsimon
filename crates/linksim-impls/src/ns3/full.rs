use parsimon_core::{
    edist::EDistBuckets,
    linksim::LinkSim,
    network::{EdgeIndex, SimNetwork},
};

#[derive(Debug)]
pub struct Ns3Full;

impl LinkSim for Ns3Full {
    fn simulate(&self, network: &SimNetwork, edge: EdgeIndex) -> EDistBuckets {
        todo!()
    }
}
