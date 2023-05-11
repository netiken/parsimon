//! Types for distributed simulations.

use std::net::SocketAddr;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    linksim::LinkSimDesc,
    network::{FctRecord, Flow, SimNetworkError},
};

/// Input parameters for worker nodes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkerParams {
    /// The name and serialized version of the link simulator.
    pub link_sim: (String, String),
    /// Link-level simulation descriptors.
    pub descs: Vec<LinkSimDesc>,
    /// All flows referenced by the descriptors.
    pub flows: Vec<Flow>,
}

/// The output of a worker.
pub type WorkerOut = Vec<(usize, Vec<FctRecord>)>;

pub(crate) async fn work_remote(
    worker: SocketAddr,
    params: WorkerParams,
) -> Result<WorkerOut, SimNetworkError> {
    // Serialize the params and send them.
    let buf = rmp_serde::encode::to_vec(&params)?;
    let mut stream = TcpStream::connect(worker).await?;
    stream.write_all(&buf).await?;

    // Read response from the remote host.
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf).await?;
    let result = rmp_serde::decode::from_slice(&buf)?;

    // Close the connection.
    stream.shutdown().await?;

    Ok(result)
}
