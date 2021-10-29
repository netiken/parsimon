use std::collections::HashMap;

use crate::{
    client::{ClientId, VNodeId},
    network::NodeId,
};

/// A mapping from clients to their node mappings
pub type ClientMap = HashMap<ClientId, NodeMap>;

/// A mapping from virtual nodes to physical nodes
pub type NodeMap = HashMap<VNodeId, NodeId>;
