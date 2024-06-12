//! Simulation constants. These are set to match the ns-3 implementation's default behavior.

use crate::units::Bytes;

/// The packet header size.
pub const SZ_PKTHDR: Bytes = Bytes::new(48);

/// The ACK size.
pub const SZ_ACK: Bytes = Bytes::new(60);
