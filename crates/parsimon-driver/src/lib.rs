use std::path::{Path, PathBuf};

use linksim_impls::ns3::full::Ns3Full;
pub use parsimon_core::network::DelayNetwork;

use parsimon_core::network::{
    types::{Link, Node},
    Flow,
};

pub fn run_from_file(spec: impl AsRef<Path>) -> Result<DelayNetwork, Error> {
    let path = spec.as_ref();
    let contents = std::fs::read_to_string(path)?;
    let spec: Spec = match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => serde_json::from_str(&contents)?,
        Some("dhall") => serde_dhall::from_str(&contents).parse()?,
        _ => return Err(Error::UnknownFileType(path.into())),
    };
    run(spec)
}

pub fn run(spec: Spec) -> Result<DelayNetwork, Error> {
    let network = match spec.linksim {
        LinkSimKind::Ns3Full { root_dir, ns3_dir } => {
            let linksim = Ns3Full::new(root_dir, ns3_dir);
            let spec = parsimon_core::Spec::builder()
                .nodes(spec.nodes)
                .links(spec.links)
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
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
    pub flows: Vec<Flow>,
    pub linksim: LinkSimKind,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum LinkSimKind {
    Ns3Full { root_dir: PathBuf, ns3_dir: PathBuf },
}
