//! Utilities for interfacing with Parsimon.

#![warn(unreachable_pub, missing_debug_implementations, missing_docs)]

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use parsimon_core::network::types::{Link, Node};
use parsimon_core::network::{Flow, Network};

/// Reads a [`Network`] from a file containing a [`TopologySpec`] in JSON or Dhall format.
pub fn read_network(topology_spec: impl AsRef<Path>) -> Result<Network, Error> {
    let spec = read_topology_spec(topology_spec)?;
    Ok(Network::new(&spec.nodes, &spec.links)?)
}

/// Reads a [`TopologySpec`] from a file in JSON or Dhall format.
pub fn read_topology_spec(path: impl AsRef<Path>) -> Result<TopologySpec, Error> {
    let contents = std::fs::read_to_string(path.as_ref())?;
    let network: TopologySpec = match path.as_ref().extension().and_then(|ext| ext.to_str()) {
        Some("json") => serde_json::from_str(&contents)?,
        Some("dhall") => serde_dhall::from_str(&contents).parse().map_err(Box::new)?,
        _ => return Err(Error::UnknownFileType(path.as_ref().into())),
    };
    Ok(network)
}

/// Read [`Flow`]s from a file in JSON format>
pub fn read_flows(path: impl AsRef<Path>) -> Result<Vec<Flow>, Error> {
    let flows: Vec<Flow> = match path.as_ref().extension().and_then(|ext| ext.to_str()) {
        Some("json") => {
            let contents = std::fs::read_to_string(path.as_ref())?;
            serde_json::from_str(&contents)?
        }
        Some("msgpack") => {
            let f = File::open(path)?;
            let reader = BufReader::new(f);
            rmp_serde::decode::from_read(reader)?
        }
        _ => return Err(Error::UnknownFileType(path.as_ref().into())),
    };
    Ok(flows)
}

/// A topology specification.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TopologySpec {
    /// Nodes.
    pub nodes: Vec<Node>,
    /// Links.
    pub links: Vec<Link>,
}

/// Error kinds for specifications and I/O.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Unknown file type.
    #[error("unknown file type: {0}")]
    UnknownFileType(PathBuf),

    /// Error serializing/deserializing Dhall.
    #[error("Dhall error")]
    Dhall(#[from] Box<serde_dhall::Error>),

    /// Error serializing/deserializing JSON.
    #[error("JSON error")]
    Json(#[from] serde_json::Error),

    /// Error serializing/deserializing MsgPack.
    #[error("MsgPack error")]
    MsgPack(#[from] rmp_serde::decode::Error),

    /// I/O error.
    #[error("IO error")]
    Io(#[from] std::io::Error),

    /// Error constructing a valid topology.
    #[error("invalid topology")]
    Topology(#[from] parsimon_core::network::TopologyError),
}
