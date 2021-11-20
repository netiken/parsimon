use parsimon_core::{
    linksim::LinkSim,
    network::{EdgeIndex, FctRecord, SimNetwork},
};

#[derive(Debug)]
pub struct Ns3Full;

impl LinkSim for Ns3Full {
    fn simulate(&self, network: &SimNetwork, edge: EdgeIndex) -> Vec<FctRecord> {
        todo!()
    }
}
