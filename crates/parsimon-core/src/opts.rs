//! This module defines the [`SimOpts`] configuration which describes how to run and process
//! link-level simulations.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::{edist::BucketOpts, units::Bytes};

/// Simulation options.
#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct SimOpts<L> {
    /// Link simulator.
    pub link_sim: L,
    /// Worker addresses.
    #[builder(default = vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)])]
    pub workers: Vec<SocketAddr>,
    /// Bucketing parameters.
    #[builder(default)]
    pub bucket_opts: BucketOpts,
    /// The maximum packet size.
    #[builder(default = Bytes::new(1000), setter(into))]
    pub sz_pktmax: Bytes,
}

impl<L> SimOpts<L> {
    pub(crate) fn is_local(&self) -> bool {
        self.workers.len() == 1 && is_localhost(self.workers[0])
    }
}

fn is_localhost(addr: SocketAddr) -> bool {
    match addr.ip() {
        IpAddr::V4(ipv4) => ipv4.is_loopback(),
        IpAddr::V6(ipv6) => ipv6.is_loopback(),
    }
}
