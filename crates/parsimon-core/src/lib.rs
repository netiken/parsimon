#![warn(unreachable_pub, missing_debug_implementations)]

//! The core Parsimon library. This crate defines [the routine](run::run) that turns a
//! specification into a [network of delay distributions](DelayNet).

#[macro_use]
mod ident;

mod client;
mod linksim;
mod mapping;
mod network;
mod spec;

mod run;

pub use client::VClient;
pub use linksim::LinkSim;
pub use mapping::{ClientMap, NodeMap};
pub use network::{DelayNet, Network};
pub use run::{run, Error};
pub use spec::Spec;
