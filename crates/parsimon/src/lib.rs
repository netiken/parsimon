//! `Parsimon` is a fast simulator for estimating flow-level tail latency distributions in data
//! center networks. Given a topology of nodes and links and a sequence of flows to simulate, it
//! produces an object which can be queried to obtain latency distributions. For more information
//! see the accompanying paper, [Scalable Tail Latency Estimates for Data Center
//! Networks](https://arxiv.org/pdf/2205.01234.pdf).

#![warn(unreachable_pub, missing_docs)]

pub mod core;
pub mod utils;
pub mod worker;

pub mod impls;
