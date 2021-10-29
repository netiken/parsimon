use petgraph::graph::DiGraph;

#[derive(Debug)]
pub struct Network {
    topology: DiGraph<Node, Channel>,
    routes: (), // copy this from cloudburst
}

impl Network {}

#[derive(Debug)]
struct Node;

#[derive(Debug)]
struct Channel;

#[derive(Debug)]
pub struct DelayNet;
