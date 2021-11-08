#![warn(unreachable_pub, missing_debug_implementations)]

//! The core Parsimon library. This crate defines [the routine](run::run) that turns a
//! specification into a [network of delay distributions](DelayNet).

#[macro_use]
mod ident;

mod client;
mod linksim;
mod network;
mod run;
mod spec;

pub(crate) mod utils;

#[cfg(test)]
pub(crate) mod testing;

// TODO: Clean these up
pub use client::{ClientId, ClientMap, NodeMap, UniqFlowId, VClient, VFlow, VNodeId};
pub use linksim::LinkSim;
pub use network::{
    topology::TopologyError,
    types::{Link, Node, NodeId, NodeKind},
    DelayNetwork, Network,
};
pub use run::{run, Error};
pub use spec::{Spec, SpecError};
