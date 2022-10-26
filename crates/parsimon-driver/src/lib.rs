use std::path::{Path, PathBuf};

use parsimon_core::network::types::{Link, Node};
use parsimon_core::network::{Flow, Network};
use parsimon_core::units::{Bytes, Nanosecs};

// TODO: probably remove me
pub fn read_network(topology_spec: impl AsRef<Path>) -> Result<Network, Error> {
    let spec = read_topology_spec(topology_spec)?;
    Ok(Network::new(&spec.nodes, &spec.links)?)
}

pub fn read_topology_spec(path: impl AsRef<Path>) -> Result<TopologySpec, Error> {
    let contents = std::fs::read_to_string(path.as_ref())?;
    let network: TopologySpec = match path.as_ref().extension().and_then(|ext| ext.to_str()) {
        Some("json") => serde_json::from_str(&contents)?,
        Some("dhall") => serde_dhall::from_str(&contents).parse()?,
        _ => return Err(Error::UnknownFileType(path.as_ref().into())),
    };
    Ok(network)
}

pub fn read_flows(path: impl AsRef<Path>) -> Result<Vec<Flow>, Error> {
    let contents = std::fs::read_to_string(path.as_ref())?;
    let flows: Vec<Flow> = match path.as_ref().extension().and_then(|ext| ext.to_str()) {
        Some("json") => serde_json::from_str(&contents)?,
        _ => return Err(Error::UnknownFileType(path.as_ref().into())),
    };
    Ok(flows)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unknown file type: {0}")]
    UnknownFileType(PathBuf),

    #[error("Dhall error")]
    Dhall(#[from] serde_dhall::Error),

    #[error("JSON error")]
    Json(#[from] serde_json::Error),

    #[error("IO error")]
    Io(#[from] std::io::Error),

    #[error("failed to run Parsimon")]
    ParsimonCore(#[from] parsimon_core::run::Error),

    #[error("invalid topology")]
    Topology(#[from] parsimon_core::network::TopologyError),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TopologySpec {
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum LinkSimKind {
    Ns3 {
        root_dir: PathBuf,
        ns3_dir: PathBuf,
        window: Bytes,
        base_rtt: Nanosecs,
    },
}
