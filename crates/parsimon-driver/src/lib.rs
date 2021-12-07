use std::fs;
use std::path::{Path, PathBuf};

use linksim_impls::ns3::full::Ns3Full;
pub use parsimon_core::network::{DelayNetwork, Flow, NodeId};
pub use parsimon_core::units::*;

use parsimon_core::network::types::{Link, Node};

pub fn run_from_files(
    network: impl AsRef<Path>,
    flows: impl AsRef<Path>,
) -> Result<DelayNetwork, Error> {
    let contents = std::fs::read_to_string(network.as_ref())?;
    let network: Network = match network.as_ref().extension().and_then(|ext| ext.to_str()) {
        Some("json") => serde_json::from_str(&contents)?,
        Some("dhall") => serde_dhall::from_str(&contents).parse()?,
        _ => return Err(Error::UnknownFileType(network.as_ref().into())),
    };
    let contents = std::fs::read_to_string(flows.as_ref())?;
    let flows: Vec<Flow> = match flows.as_ref().extension().and_then(|ext| ext.to_str()) {
        Some("json") => serde_json::from_str(&contents)?,
        _ => return Err(Error::UnknownFileType(flows.as_ref().into())),
    };
    let spec = Spec { network, flows };
    run(spec)
}

pub fn run(spec: Spec) -> Result<DelayNetwork, Error> {
    let network = match spec.network.linksim {
        LinkSimKind::Ns3Full { root_dir, ns3_dir } => {
            fs::create_dir_all(&root_dir)?;
            let root_dir = fs::canonicalize(root_dir)?;
            let ns3_dir = fs::canonicalize(ns3_dir)?;
            let linksim = Ns3Full::new(root_dir, ns3_dir);
            let spec = parsimon_core::Spec::builder()
                .nodes(spec.network.nodes)
                .links(spec.network.links)
                .flows(spec.flows)
                .linksim(linksim)
                .build();
            parsimon_core::run(spec)?
        }
    };
    Ok(network)
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
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Spec {
    pub network: Network,
    pub flows: Vec<Flow>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Network {
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
    pub linksim: LinkSimKind,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum LinkSimKind {
    Ns3Full { root_dir: PathBuf, ns3_dir: PathBuf },
}
