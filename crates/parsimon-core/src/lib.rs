#![warn(unreachable_pub, missing_debug_implementations)]

//! The core Parsimon library. This crate defines [the routine](run::run) that turns a
//! specification into a [network of delay distributions](network::DelayNetwork).

#[macro_use]
mod ident;

pub mod edist;
pub mod linksim;
pub mod network;
pub mod pruner;
pub mod run;
pub mod spec;
pub mod units;

pub(crate) mod utils;

pub mod testing;
