//! This crate implements the behavior of worker nodes for distributed Parsimon simulations.

#![warn(unreachable_pub, missing_debug_implementations, missing_docs)]

mod worker;

pub use worker::start;
