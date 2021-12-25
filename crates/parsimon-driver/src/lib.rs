use std::fs;
use std::path::{Path, PathBuf};

use linksim_impls::ns3::full::Ns3Full;
use parsimon_core::cluster::DefaultClustering;
pub use parsimon_core::network::{DelayNetwork, Flow, FlowId, NodeId};
pub use parsimon_core::units::*;

use parsimon_core::network::types::{Link, Node};

pub fn run_from_files(
    network: impl AsRef<Path>,
    flows: impl AsRef<Path>,
) -> Result<DelayNetwork, Error> {
    let network = read_network_spec(network)?;
    let flows = read_flows(flows)?;
    run(network, flows)
}

pub fn run(network: NetworkSpec, flows: Vec<Flow>) -> Result<DelayNetwork, Error> {
    let spec = parsimon_core::Spec::builder()
        .nodes(network.nodes)
        .links(network.links)
        .flows(flows)
        .build();
    let network = match network.linksim {
        LinkSimKind::Ns3Full {
            root_dir,
            ns3_dir,
            window,
        } => {
            fs::create_dir_all(&root_dir)?;
            let linksim = Ns3Full::new(root_dir, ns3_dir, window);
            parsimon_core::run(spec, linksim, DefaultClustering)?
        }
    };
    Ok(network)
}

pub fn read_network_spec(path: impl AsRef<Path>) -> Result<NetworkSpec, Error> {
    let contents = std::fs::read_to_string(path.as_ref())?;
    let network: NetworkSpec = match path.as_ref().extension().and_then(|ext| ext.to_str()) {
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
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct NetworkSpec {
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
    pub linksim: LinkSimKind,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum LinkSimKind {
    Ns3Full {
        root_dir: PathBuf,
        ns3_dir: PathBuf,
        window: Bytes,
    },
}
