#![allow(missing_docs)]
//! Types for representing units.

macro_rules! unit {
    ($name: ident) => {
        #[derive(
            Debug,
            Default,
            Copy,
            Clone,
            PartialOrd,
            Ord,
            PartialEq,
            Eq,
            Hash,
            derive_more::Add,
            derive_more::Sub,
            derive_more::AddAssign,
            derive_more::SubAssign,
            derive_more::Sum,
            derive_more::FromStr,
            serde::Serialize,
            serde::Deserialize,
        )]
        pub struct $name(u64);

        impl $name {
            pub const ZERO: $name = Self::new(0);
            pub const ONE: $name = Self::new(1);
            pub const MAX: $name = Self::new(u64::MAX);

            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            pub const fn into_u64(self) -> u64 {
                self.0
            }

            pub const fn into_f64(self) -> f64 {
                self.0 as f64
            }

            pub const fn into_usize(self) -> usize {
                self.0 as usize
            }

            pub fn scale_by(self, val: f64) -> Self {
                let inner = self.0 as f64 * val;
                Self(inner.round() as u64)
            }
        }

        impl From<$name> for f64 {
            fn from(val: $name) -> Self {
                val.into_f64()
            }
        }
    };
}

unit!(Gbps);

impl std::fmt::Display for Gbps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}Gbps", self.0)
    }
}

unit!(Mbps);

impl std::fmt::Display for Mbps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}Mbps", self.0)
    }
}

unit!(BitsPerSec);

impl BitsPerSec {

    #[allow(non_snake_case)]
    pub fn length(&self, size: Bytes) -> Nanosecs {
        assert!(*self != BitsPerSec::ZERO);
        if size == Bytes::ZERO {
            return Nanosecs::ZERO;
        }
        let bytes = size.into_f64();
        let bps = self.into_f64();
        let delta = (bytes * 1e9 * 8.0) / bps;
        let delta = delta.round() as u64;
        Nanosecs::new(delta)
    }

    #[allow(non_snake_case)]
    pub fn width(&self, delta: Nanosecs) -> Bytes {
        if delta == Nanosecs::ZERO {
            return Bytes::ZERO;
        }
        let delta = delta.into_f64();
        let bps = self.into_f64();
        let size = (bps * delta) / (1e9 * 8.0);
        let size = size.round() as u64;
        Bytes::new(size)
    }
}

impl std::fmt::Display for BitsPerSec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}bps", self.0)
    }
}

impl From<Gbps> for BitsPerSec {
    fn from(val: Gbps) -> Self {
        Self::new(val.0 * 1_000_000_000)
    }
}

impl From<Mbps> for BitsPerSec {
    fn from(val: Mbps) -> Self {
        Self::new(val.0 * 1_000_000)
    }
}

unit!(Secs);

impl std::fmt::Display for Secs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}s", self.0)
    }
}

unit!(Millisecs);

impl std::fmt::Display for Millisecs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}ms", self.0)
    }
}

unit!(Microsecs);

impl std::fmt::Display for Microsecs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}us", self.0)
    }
}

unit!(Nanosecs);

impl std::fmt::Display for Nanosecs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}ns", self.0)
    }
}

impl From<Secs> for Nanosecs {
    fn from(s: Secs) -> Self {
        Self::new(s.0 * 1_000_000_000)
    }
}

impl From<Millisecs> for Nanosecs {
    fn from(ms: Millisecs) -> Self {
        Self::new(ms.0 * 1_000_000)
    }
}

impl From<Microsecs> for Nanosecs {
    fn from(us: Microsecs) -> Self {
        Self::new(us.0 * 1_000)
    }
}

unit!(Gigabytes);

impl std::fmt::Display for Gigabytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}GB", self.0)
    }
}

unit!(Kilobytes);

impl std::fmt::Display for Kilobytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}KB", self.0)
    }
}

unit!(Bytes);

impl std::fmt::Display for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}B", self.0)
    }
}

impl From<Gigabytes> for Bytes {
    fn from(gb: Gigabytes) -> Self {
        Bytes::new(gb.0 * 1_000_000_000)
    }
}

impl From<Kilobytes> for Bytes {
    fn from(kb: Kilobytes) -> Self {
        Bytes::new(kb.0 * 1_000)
    }
}
