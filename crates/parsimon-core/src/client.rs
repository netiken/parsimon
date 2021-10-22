use crate::network::Flow;

identifier!(ClientId, usize);

#[derive(Debug, derive_new::new)]
pub struct Client {
    pub id: ClientId,
    name: String,
    flows: Vec<Flow>,
}

impl Client {
    /// Get a reference to the client's name.
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Get a reference to the client's flows.
    pub fn flows(&self) -> &[Flow] {
        self.flows.as_ref()
    }

    /// Get a mutable reference to the client's flows.
    pub fn flows_mut(&mut self) -> &mut Vec<Flow> {
        &mut self.flows
    }
}
