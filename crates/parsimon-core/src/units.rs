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
        }
    };
}

unit!(Gbps);

impl std::fmt::Display for Gbps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}Gbps", self.0)
    }
}

unit!(Nanosecs);

impl std::fmt::Display for Nanosecs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}ns", self.0)
    }
}

unit!(Bytes);

impl std::fmt::Display for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}B", self.0)
    }
}
