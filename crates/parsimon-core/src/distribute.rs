//! Types for distributed simulations.

use std::path::Path;
use std::{net::SocketAddr, path::PathBuf};

use openssh::{KnownHosts, Session, Stdio};
use openssh_sftp_client::Sftp;
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
    /// The file path of the workload chunk.
    pub chunk_path: PathBuf,
}

/// A chunk of work.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkerChunk {
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
    chunk: WorkerChunk,
) -> Result<WorkerOut, SimNetworkError> {
    // Send the chunk first.
    send_chunk(worker, chunk, &params.chunk_path).await?;

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

async fn send_chunk(
    worker: SocketAddr,
    chunk: WorkerChunk,
    path: &Path,
) -> Result<(), SimNetworkError> {
    // Start an SSH session with SFTP.
    let session = Session::connect_mux(worker.ip().to_string(), KnownHosts::Strict).await?;
    let mut child = session
        .subsystem("sftp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .await?;
    let sftp = Sftp::new(
        child.stdin().take().unwrap(),
        child.stdout().take().unwrap(),
        Default::default(),
    )
    .await?;

    // Serialize the worker chunk.
    let buf = rmp_serde::encode::to_vec(&chunk)?;

    // Write it to the remote host.
    let mut file = sftp.create(path).await?;
    file.write_all(&buf).await?;
    file.close().await?;

    sftp.close().await?;
    Ok(())
}
