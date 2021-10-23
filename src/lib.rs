#![warn(unreachable_pub, missing_debug_implementations)]

#[macro_use]
mod ident;

pub mod client;
pub mod delay;
pub mod linksim;
pub mod mapping;
pub mod network;

use client::VClient;
use delay::DelayNet;
use mapping::ClientMap;
use network::Network;

pub fn tbd(network: Network, clients: &[VClient], mappings: ClientMap) -> Result<DelayNet, Error> {
    todo!()
}

#[derive(Debug)]
pub enum Error {}
