use crate::network::SimNetwork;

pub trait Pruner {
    fn prune(&self, network: SimNetwork) -> SimNetwork;
}
