# `Parsimon`

`Parsimon` is a fast simulator for estimating flow-level tail latency
distributions in data center networks. For more information see
[the accompanying paper](https://arxiv.org/pdf/2205.01234.pdf).

The simulator is implemented as a set of Rust libraries, with its main entry
point in the `crates/parsimon-core` library. To get started, first make sure
[Rust is installed](https://www.rust-lang.org/tools/install), then run

```bash
$ cd crates/parsimon-core
$ cargo doc --open
```
