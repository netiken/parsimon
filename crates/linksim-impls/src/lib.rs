//! This crate contains implementations of the [`LinkSim`](parsimon_core::linksim::LinkSim) trait.
//! The types here bridge Parsimon and its backend link-level simulators.

#![warn(unreachable_pub, missing_debug_implementations, missing_docs)]

pub mod minim;
pub mod ns3;

pub use crate::minim::MinimLink;
pub use crate::ns3::Ns3Link;
