# `Parsimon`

`Parsimon` is a fast simulator for estimating flow-level tail latency
distributions in data center networks. Given a topology of nodes and links and
a sequence of flows to simulate, it produces an object which can be queried to
obtain latency distributions. For more information see the accompanying paper,
[Scalable Tail Latency Estimates for Data Center Networks](https://arxiv.org/pdf/2205.01234.pdf).

## Overview

Given a topology and a workload, `Parsimon`

1. decomposes the large network simulation into component link simulations,
2. clusters similar link simulations together,
3. configures and runs one link simulation per cluster, and
4. aggregates results to produce end-to-end, full-network estimates.

These techniques are implemented as a set of Rust libraries. Steps 1 and 4 can
be found in `crates/parsimon-core`, step 2 is implemented in
`crates/clustering-impls`, and the configuration of link simulations in step 3
is in `crates/linksim-impls`.

## Getting started

First, make sure [Rust is installed](https://www.rust-lang.org/tools/install).
Then, clone this repository and its submodules. The `Parsimon` libraries are
all located in the `crates` directory. To obtain the full documentation for any
library, `cd` into its directory and run `cargo doc --open`. We recommend
starting with `parsimon`, which re-exports public types from all other crates:

```bash
$ cd crates/parsimon
$ cargo doc --open
```

This should open a browser tab with documentation for the library and its
dependencies.
