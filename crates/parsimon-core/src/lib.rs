#![warn(unreachable_pub, missing_debug_implementations, missing_docs)]

//! The core Parsimon library. This crate defines [run::run()], which turns a
//! [specification](Spec) into a [network of delay distributions](network::DelayNetwork).

#[macro_use]
mod ident;

pub mod cluster;
pub mod constants;
pub mod distribute;
pub mod edist;
pub mod linksim;
pub mod network;
pub mod opts;
pub mod run;
pub mod spec;
pub mod units;
pub mod routing;

pub(crate) mod utils;

pub mod testing;

pub use run::{run, Error};
pub use spec::Spec;
