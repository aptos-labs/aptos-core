// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file contains custom scalar types that correspond to the Move primitive types.
//! These are used anywhere we use GraphQL to represent Move resources, e.g. as
//! returned by the API, in indexer processors, and in ABI-based codegen.
//!
//! All types in this crate must impl serde Deserialize and Serialize.

use move_core_types::account_address::AccountAddress;
use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub static ALL_CUSTOM_SCALARS_TYPE_NAMES: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        U8::type_name(),
        U16::type_name(),
        U32::type_name(),
        U64::type_name(),
        U128::type_name(),
        U256::type_name(),
        Address::type_name(),
        Any::type_name(),
    ]
});

pub type U8 = u8;
pub type U16 = u16;
pub type U32 = u32;

/// Represents an AccountAddress.
pub type Address = AccountAddress;

/// This is a custom scalar that represents a blob of Move data that we haven't been
/// able to fully parse out. Ideally we never need this but for now we don't support
/// generic type params, so we represent them as this. This way the downstream client
/// code generators can cast this to Any rather than something inaccurate like String.
pub type Any = serde_json::Value;

// We encode u64, u128, and u256 as strings. These types accept them as strings but
// represent them internally as actual number types.

macro_rules! define_integer_type {
    ($n:ident, $t:ty, $d:literal) => {
        #[doc = $d]
        #[derive(Clone, Debug, Default, Eq, PartialEq, Copy)]
        pub struct $n(pub $t);

        impl From<$t> for $n {
            fn from(d: $t) -> Self {
                Self(d)
            }
        }

        impl From<$n> for $t {
            fn from(d: $n) -> Self {
                d.0
            }
        }

        impl std::fmt::Display for $n {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{}", &self.0)
            }
        }

        impl Serialize for $n {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.0.to_string().serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $n {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let s = <String>::deserialize(deserializer)?;
                s.parse().map_err(serde::de::Error::custom)
            }
        }

        impl std::str::FromStr for $n {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let data = s.parse::<$t>().map_err(|e| {
                    anyhow::format_err!(
                        "Parsing string {:?} into type {} failed, caused by error: {}",
                        s,
                        stringify!($t),
                        e
                    )
                })?;

                Ok($n(data))
            }
        }
    };
}

define_integer_type!(U64, u64, "u64 encoded as a string.");
define_integer_type!(U128, u128, "u128 encoded as a string.");
define_integer_type!(
    U256,
    move_core_types::u256::U256,
    "u256 encoded as a string."
);

/// The schema generator needs to reference these types as strings. This trait
/// facilitates that by having every type have a string representation. We cannot use
/// std::any::type_name::<T>() because it "sees through" type alises to the
/// underlying type. In other words, for U8 it would return "u8".
pub trait TypeName {
    /// Get the name of the type as a string.
    fn type_name() -> &'static str;
}

impl TypeName for U8 {
    fn type_name() -> &'static str {
        "U8"
    }
}

impl TypeName for U16 {
    fn type_name() -> &'static str {
        "U16"
    }
}

impl TypeName for U32 {
    fn type_name() -> &'static str {
        "U32"
    }
}

impl TypeName for U64 {
    fn type_name() -> &'static str {
        "U64"
    }
}

impl TypeName for U128 {
    fn type_name() -> &'static str {
        "U128"
    }
}

impl TypeName for U256 {
    fn type_name() -> &'static str {
        "U256"
    }
}

impl TypeName for Address {
    fn type_name() -> &'static str {
        "Address"
    }
}

impl TypeName for Any {
    fn type_name() -> &'static str {
        "Any"
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_type_name() {
        assert_eq!(U8::type_name(), "U8");
        assert_eq!(Address::type_name(), "Address");
    }

    #[test]
    fn test_serde_u16() {
        let num = 12345;
        let serialized = serde_json::to_string(&num).unwrap();
        assert_eq!(serialized, "12345");
        let deserialized: U16 = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, num);
    }

    #[test]
    fn test_serde_u128() {
        let num = U128(123456789012345678901234567890);
        let serialized = serde_json::to_string(&num).unwrap();
        assert_eq!(serialized, "\"123456789012345678901234567890\"");
        let deserialized: U128 = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, num);
    }
}
