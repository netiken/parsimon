use parsimon_core::{
    network::{Channel, EdgeIndex, SimNetwork},
    units::{BitsPerSec, Bytes},
};

const PKTSIZE_MAX: Bytes = Bytes::new(1000);
const NS3_ACKSIZE: Bytes = Bytes::new(60);

pub(crate) fn ack_rate(network: &SimNetwork, edge: EdgeIndex) -> BitsPerSec {
    let forward_chan = network.edge(edge).unwrap();
    let reverse_edge = network
        .find_edge(forward_chan.dst(), forward_chan.src())
        .unwrap();
    let flows = network.flows_on(reverse_edge).unwrap();
    let nr_ack_bytes = flows
        .iter()
        .map(|f| {
            let nr_pkts = (f.size.into_f64() / PKTSIZE_MAX.into_f64()).ceil();
            NS3_ACKSIZE.scale_by(nr_pkts)
        })
        .sum::<Bytes>();
    let duration = flows.last().map(|f| f.start).unwrap_or_default()
        - flows.first().map(|f| f.start).unwrap_or_default();
    let inner = nr_ack_bytes.into_f64() * 8.0 * 1e9 / duration.into_f64();
    BitsPerSec::new(inner.round() as u64)
}
