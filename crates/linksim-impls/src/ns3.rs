use std::{
    collections::HashMap,
    num::ParseIntError,
    path::{Path, PathBuf},
};

use parsimon_core::{
    linksim::LinkSimError,
    network::{types::TracedChannel, FctRecord, NodeId, SimNetwork},
    units::Bytes,
};

// We can implement `LinkSim` for any type that implements the `Ns3Sim` trait, defined below
macro_rules! linksim_impl {
    ($ty: ty) => {
        impl LinkSim for $ty {
            fn simulate(&self, network: &SimNetwork, edge: EdgeIndex) -> LinkSimResult {
                let mk_path = |dir, file| [dir, file].into_iter().collect::<PathBuf>();

                // Prepare directory
                let chan = network.edge(edge).ok_or(LinkSimError::UnknownEdge(edge))?;
                let dir = self.dir_for((chan.src(), chan.dst()));
                fs::create_dir_all(&dir)?;

                // Write the topology
                let topology = <$ty>::to_ns3_topology(network, chan);
                fs::write(mk_path(dir.as_path(), "topology.txt".as_ref()), &topology)?;

                // Write the flows
                let flows = <$ty>::to_ns3_flows(network, chan)?;
                fs::write(mk_path(dir.as_path(), "flows.txt".as_ref()), &flows)?;

                // Run the simulation
                self.run_ns3(&dir)?;

                // Read results and return them
                let s = fs::read_to_string(mk_path(
                    dir.as_path(),
                    "fct_topology_flows_dctcp.txt".as_ref(),
                ))?;
                let records = <$ty>::from_ns3_records(&s).map_err(|e| anyhow::anyhow!(e))?;
                Ok(records)
            }
        }
    };
}

pub mod full;

trait Ns3Sim {
    fn root_dir(&self) -> &Path;

    fn ns3_dir(&self) -> &Path;

    fn window(&self) -> Bytes;

    fn to_ns3_topology(network: &SimNetwork, chan: &TracedChannel) -> String;

    fn to_ns3_flows(network: &SimNetwork, chan: &TracedChannel) -> Result<String, LinkSimError> {
        let flow_map = network
            .flows()
            .map(|f| (f.id, f))
            .collect::<HashMap<_, _>>();
        let nr_flows = chan.nr_flows();
        // First line: # of flows
        // src0 dst0 3 dst_port0 size0 start_time0
        // src1 dst1 3 dst_port1 size1 start_time1
        let lines = std::iter::once(nr_flows.to_string())
            .chain(chan.flows().map(|id| {
                let f = *flow_map.get(&id).unwrap();
                format!(
                    "{} {} {} 3 100 {} {}",
                    f.id,
                    f.src,
                    f.dst,
                    f.size.into_u64(),
                    f.start.into_u64() as f64 / 1e9 // in seconds, for some reason
                )
            }))
            .collect::<Vec<_>>();
        Ok(lines.join("\n"))
    }

    fn from_ns3_records(s: &str) -> Result<Vec<FctRecord>, ParseNs3Error> {
        s.lines().map(|l| FctRecord::from_ns3(l)).collect()
    }

    fn dir_for(&self, (src, dst): (NodeId, NodeId)) -> PathBuf {
        let mut dir = PathBuf::new();
        dir.push(self.root_dir());
        dir.push(format!("{}-{}", src, dst));
        dir
    }

    fn run_ns3(&self, dir: impl AsRef<Path>) -> cmd_lib::CmdResult {
        let dir = dir.as_ref();
        let ns3_dir = self.ns3_dir();
        let window = self.window().into_u64().to_string();
        let extra_args = &[
            "--topo", "topology", "--trace", "flows", "--bw", "100", "--cc", "dctcp",
        ];
        cmd_lib::run_cmd! {
            cd ${ns3_dir};
            python2 run.py --root ${dir} --fwin ${window} $[extra_args] > ${dir}/output.txt 2>&1
        }
    }
}

trait FromNs3: Sized {
    fn from_ns3(s: &str) -> Result<Self, ParseNs3Error>;
}

impl FromNs3 for FctRecord {
    fn from_ns3(s: &str) -> Result<Self, ParseNs3Error> {
        // sip, dip, sport, dport, size (B), start_time, fct (ns), standalone_fct (ns)
        const NR_NS3_FIELDS: usize = 9;
        let fields = s.split_whitespace().collect::<Vec<_>>();
        let nr_fields = fields.len();
        if nr_fields != NR_NS3_FIELDS {
            return Err(ParseNs3Error::WrongNrFields {
                expected: NR_NS3_FIELDS,
                got: nr_fields,
            });
        }
        Ok(FctRecord {
            id: fields[0].parse()?,
            size: fields[5].parse()?,
            start: fields[6].parse()?,
            fct: fields[7].parse()?,
            ideal: fields[8].parse()?,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum ParseNs3Error {
    #[error("Wrong number of fields (expected {expected}, got {got}")]
    WrongNrFields { expected: usize, got: usize },

    #[error("Failed to parse field")]
    ParseInt(#[from] ParseIntError),
}
