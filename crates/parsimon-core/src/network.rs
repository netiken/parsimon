mod routing;

use petgraph::graph::DiGraph;

use self::routing::Routes;

type Topology = DiGraph<Node, Channel>;

#[derive(Debug)]
pub struct Network {
    topology: Topology,
    routes: Routes,
}

impl Network {
    // TODO: Figure out a way to create a topology. Probably we'll want a newtype
    pub fn new(topology: Topology) -> Self {
        let routes = Routes::new(&topology);
        Self { topology, routes }
    }
}

#[derive(Debug)]
pub struct Node {
    id: NodeId,
    kind: NodeKind,
}

impl Node {
    pub fn new_host(id: NodeId) -> Self {
        Self {
            id,
            kind: NodeKind::Host,
        }
    }

    pub fn new_switch(id: NodeId) -> Self {
        Self {
            id,
            kind: NodeKind::Switch,
        }
    }
}

#[derive(Debug)]
pub enum NodeKind {
    Host,
    Switch,
}

identifier!(NodeId, usize);

#[derive(Debug)]
pub struct Channel;

#[derive(Debug)]
pub struct DelayNet;
