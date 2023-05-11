//! This module defines the worker functionality, which consists of listening to requests, and
//! responding with results from a simulation

use std::{
    io::{BufReader, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use anyhow::Context;
use linksim_impls::{minim::MinimLink, ns3::Ns3Link};
use parsimon_core::{
    distribute::WorkerParams,
    linksim::{LinkSim, LinkSimError, LinkSimSpec},
    network::{FctRecord},
};
use rayon::prelude::*;
use rmp_serde::decode;
use rustc_hash::FxHashMap;

/// Starts a worker on a port.
pub fn start(port: u16) -> anyhow::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    let listener_thread = thread::spawn(move || serve(running, port));

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .with_context(|| "failed to set interrupt handler")?;

    // Wait for the listener thread to finish
    listener_thread
        .join()
        .unwrap()
        .with_context(|| "error in parsimon_worker::serve")?;
    Ok(())
}

fn serve(running: Arc<AtomicBool>, port: u16) -> anyhow::Result<()> {
    let addr: SocketAddr = format!("0.0.0.0:{port}").parse().unwrap();
    let listener = TcpListener::bind(addr).with_context(|| "failed to bind listener")?;
    listener
        .set_nonblocking(true)
        .with_context(|| "failed to set listener as nonblocking")?;
    while running.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _addr)) => {
                thread::spawn(move || {
                    handle_client(stream).unwrap();
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => anyhow::bail!(e),
        }
    }
    Ok(())
}

fn handle_client(mut stream: TcpStream) -> anyhow::Result<()> {
    let params: WorkerParams = decode::from_read(BufReader::new(&stream))?;
    let sim_name = &params.link_sim.0[..];
    let sim_ser = &params.link_sim.1[..];
    let results = match sim_name {
        "minim" => {
            let sim: MinimLink = serde_json::from_str(sim_ser)?;
            simulate_chunk(params, sim)?
        }
        "ns3" => {
            let sim: Ns3Link = serde_json::from_str(sim_ser)?;
            simulate_chunk(params, sim)?
        }
        _ => unimplemented!("unknown link simulator"),
    };
    let buf = rmp_serde::encode::to_vec(&results)?;
    stream.write_all(&buf)?;
    stream.flush()?;
    Ok(())
}

fn simulate_chunk<S>(
    params: WorkerParams,
    sim: S,
) -> Result<Vec<(usize, Vec<FctRecord>)>, LinkSimError>
where
    S: LinkSim + Sync,
{
    let id2flow = params
        .flows
        .iter()
        .map(|f| (f.id, f.to_owned()))
        .collect::<FxHashMap<_, _>>();
    let (s, r) = crossbeam_channel::unbounded();
    params
        .descs
        .into_par_iter()
        .try_for_each_with(s, |s, desc| {
            let flows = desc
                .flows
                .iter()
                .map(|id| id2flow.get(id).unwrap().to_owned())
                .collect::<Vec<_>>();
            let spec = LinkSimSpec {
                edge: desc.edge,
                bottleneck: desc.bottleneck,
                other_links: desc.other_links,
                nodes: desc.nodes,
                flows,
            };
            let data = sim.simulate(spec)?;
            s.send((desc.edge, data)).unwrap(); // the channel should never become disconnected
            Result::<(), LinkSimError>::Ok(())
        })?;
    Ok(r.iter().collect())
}
