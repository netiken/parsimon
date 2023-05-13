//! An interface to a link-level simulator built atop ns-3. This hooks into the ns-3 implementation
//! at <https://github.com/kwzhao/High-Precision-Congestion-Control>, which is assumed to be
//! downloaded and compiled prior to the use of this type.

use std::path::PathBuf;

use ns3_frontend::{CcKind, Ns3Simulation};
use parsimon_core::{
    linksim::{LinkSim, LinkSimResult, LinkSimSpec},
    units::{Bytes, Nanosecs},
};

/// An ns-3 link simulation.
#[derive(Debug, typed_builder::TypedBuilder, serde::Serialize, serde::Deserialize)]
pub struct Ns3Link {
    /// The top-level directory where data files will be written.
    #[builder(setter(into))]
    pub root_dir: PathBuf,
    /// The path to the ns-3 simulator (`{path_to}/High-Precision-Congestion-Control/simulation`)
    #[builder(setter(into))]
    pub ns3_dir: PathBuf,
    /// The sending window.
    #[builder(setter(into))]
    pub window: Bytes,
    /// The base round-trip time.
    #[builder(setter(into))]
    pub base_rtt: Nanosecs,
    /// The congestion control algorithm.
    #[builder(default)]
    pub cc_kind: CcKind,
}

impl LinkSim for Ns3Link {
    fn name(&self) -> String {
        "ns3".into()
    }

    fn simulate(&self, spec: LinkSimSpec) -> LinkSimResult {
        let (bsrc, bdst) = (spec.bottleneck.from, spec.bottleneck.to);
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
