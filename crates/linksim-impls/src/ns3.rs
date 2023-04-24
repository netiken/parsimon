//! An interface to a link-level simulator built atop ns-3.

use std::path::PathBuf;

use ns3_frontend::{CcKind, Ns3Simulation};
use parsimon_core::{
    linksim::{LinkSim, LinkSimResult, LinkSimSpec},
    units::{Bytes, Nanosecs},
};

/// An ns-3 link simulation.
#[derive(Debug, typed_builder::TypedBuilder)]
pub struct Ns3Link {
    #[builder(setter(into))]
    root_dir: PathBuf,
    #[builder(setter(into))]
    ns3_dir: PathBuf,
    #[builder(setter(into))]
    window: Bytes,
    #[builder(setter(into))]
    base_rtt: Nanosecs,
    #[builder(default)]
    cc_kind: CcKind,
}

impl LinkSim for Ns3Link {
    fn simulate(&self, spec: LinkSimSpec) -> LinkSimResult {
        let (bsrc, bdst) = (spec.bottleneck.a, spec.bottleneck.b);
        let (spec, _) = spec.contiguousify();

        // Set up and run simulation
        let mut data_dir = PathBuf::from(&self.root_dir);
        data_dir.push(format!("{bsrc}-{bdst}"));
        let sim = Ns3Simulation::builder()
            .ns3_dir(&self.ns3_dir)
            .data_dir(data_dir)
            .nodes(spec.generic_nodes().collect())
            .links(spec.generic_links().collect())
            .window(self.window)
            .base_rtt(self.base_rtt)
            .cc_kind(self.cc_kind)
            .flows(spec.flows)
            .build();
        let records = sim.run().map_err(|e| anyhow::anyhow!(e))?;
        Ok(records)
    }
}
