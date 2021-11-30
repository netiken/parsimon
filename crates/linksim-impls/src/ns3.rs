use std::{
    collections::HashMap,
    fs, io,
    num::ParseIntError,
    path::{Path, PathBuf},
};

use parsimon_core::{
    linksim::LinkSimError,
    network::{types::TracedChannel, FctRecord, SimNetwork},
};

// We can implement `LinkSim` for any type that implements the `Ns3Sim` trait, defined below
macro_rules! linksim_impl {
    ($ty: ty) => {
        impl LinkSim for $ty {
            fn simulate(&self, network: &SimNetwork, edge: EdgeIndex) -> LinkSimResult {
                fs::create_dir_all(&self.root_dir())?;
                let chan = network.edge(edge).ok_or(LinkSimError::UnknownEdge(edge))?;
                let topology = <$ty>::to_ns3_topology(network, chan);
                self.write("topology.txt", &topology)?;
                let flows = <$ty>::to_ns3_flows(network, chan)?;
                self.write("flows.txt", &flows)?;
                self.run_ns3()?;
                let s = self.read("fct_topology_flows_dctcp.txt")?;
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
                    "{} {} 3 100 {} {}",
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

    fn read(&self, file: impl AsRef<Path>) -> io::Result<String> {
        let path = [self.root_dir(), file.as_ref()]
            .into_iter()
            .collect::<PathBuf>();
        fs::read_to_string(path)
    }

    fn write(&self, file: impl AsRef<Path>, contents: &str) -> io::Result<()> {
        let path = [self.root_dir(), file.as_ref()]
            .into_iter()
            .collect::<PathBuf>();
        fs::write(path, contents)
    }

    fn run_ns3(&self) -> cmd_lib::CmdResult {
        let root_dir = self.root_dir();
        let ns3_dir = self.ns3_dir();
        let extra_args = &[
            "--topo", "topology", "--trace", "flows", "--bw", "100", "--cc", "dctcp",
        ];
        cmd_lib::run_cmd! {
            cd ${ns3_dir};
            python2 run.py --root ${root_dir} $[extra_args] > ${root_dir}/output.txt 2>&1
        }
    }
}

trait FromNs3: Sized {
    fn from_ns3(s: &str) -> Result<Self, ParseNs3Error>;
}

impl FromNs3 for FctRecord {
    fn from_ns3(s: &str) -> Result<Self, ParseNs3Error> {
        // sip, dip, sport, dport, size (B), start_time, fct (ns), standalone_fct (ns)
        const NR_NS3_FIELDS: usize = 8;
        let fields = s.split_whitespace().collect::<Vec<_>>();
        let nr_fields = fields.len();
        if nr_fields != NR_NS3_FIELDS {
            return Err(ParseNs3Error::WrongNrFields {
                expected: NR_NS3_FIELDS,
                got: nr_fields,
            });
        }
        Ok(FctRecord {
            size: fields[4].parse()?,
            start: fields[5].parse()?,
            fct: fields[6].parse()?,
            ideal: fields[7].parse()?,
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
