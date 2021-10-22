#![warn(unreachable_pub, missing_debug_implementations)]

pub mod client;
pub mod ident;
pub mod linksim;
pub mod mapping;
pub mod network;

use std::collections::HashMap;

use client::Client;
use mapping::ClientMap;
use network::Network;

pub fn tbd(network: Network, clients: &[Client], mappings: ClientMap) -> DelayNet {
    todo!()
}

pub struct DelayNet;
