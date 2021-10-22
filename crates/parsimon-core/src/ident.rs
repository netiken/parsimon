macro_rules! identifier {
    ($name: ident, $inner: ty) => {
        #[allow(missing_docs)]
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
            derive_more::Display,
            derive_more::FromStr,
            serde::Serialize,
            serde::Deserialize,
        )]
        pub struct $name($inner);

        impl $name {
            /// ID 0.
            pub const ZERO: $name = Self::new(0);

            /// ID 1.
            pub const ONE: $name = Self::new(1);

            /// Creates a new ID.
            pub const fn new(val: $inner) -> Self {
                Self(val)
            }

            /// Returns the inner representation of the ID.
            pub const fn inner(self) -> $inner {
                self.0
            }
        }
    };
}
