// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implemented of unsigned and signed 256 bit integers.
//!
//! This uses the `ethnum` crate for the underlying representation. This is one of the
//! most downloaded 256 bit implementation for Rust, and has full integration of both
//! signed and unsigned integers with the standard Rust int types. This module is
//! merely a wrapper around the provided types.

use num::{bigint::Sign, BigInt};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    ops::{BitAnd, BitOr, BitXor, Neg, Shl, Shr},
    str::FromStr,
};

// ---- U256 Representation

#[derive(Clone, Copy, Default, Hash, PartialOrd, PartialEq, Ord, Eq)]
pub struct U256 {
    repr: ethnum::U256,
}

impl U256 {
    pub const MAX: U256 = U256 {
        repr: ethnum::U256::MAX,
    };
    pub const MIN: U256 = U256 {
        repr: ethnum::U256::MIN,
    };
    pub const ONE: U256 = U256 {
        repr: ethnum::U256::ONE,
    };
    pub const ZERO: U256 = U256 {
        repr: ethnum::U256::ZERO,
    };

    #[inline(always)]
    pub fn repr(self) -> ethnum::U256 {
        self.repr
    }

    #[inline(always)]
    pub fn from_le_bytes(bytes: [u8; 32]) -> Self {
        Self {
            repr: ethnum::U256::from_le_bytes(bytes),
        }
    }

    #[inline(always)]
    pub fn to_le_bytes(self) -> [u8; 32] {
        self.repr.to_le_bytes()
    }
}

impl From<ethnum::U256> for U256 {
    #[inline(always)]
    fn from(repr: ethnum::U256) -> Self {
        Self { repr }
    }
}

impl From<U256> for ethnum::U256 {
    #[inline(always)]
    fn from(value: U256) -> Self {
        value.repr
    }
}

impl From<U256> for BigInt {
    fn from(value: U256) -> Self {
        BigInt::from_bytes_le(Sign::Plus, &value.to_le_bytes())
    }
}

impl std::fmt::Debug for U256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "U256({})", self.repr)
    }
}

// ---- I256 Representation

#[derive(Clone, Copy, Default, Hash, PartialOrd, PartialEq, Ord, Eq)]
pub struct I256 {
    repr: ethnum::I256,
}

impl I256 {
    pub const MAX: I256 = I256 {
        repr: ethnum::I256::MAX,
    };
    pub const MIN: I256 = I256 {
        repr: ethnum::I256::MIN,
    };
    pub const ONE: I256 = I256 {
        repr: ethnum::I256::ONE,
    };
    pub const ZERO: I256 = I256 {
        repr: ethnum::I256::ZERO,
    };

    #[inline(always)]
    pub fn repr(self) -> ethnum::I256 {
        self.repr
    }

    #[inline(always)]
    pub fn from_le_bytes(bytes: [u8; 32]) -> Self {
        Self {
            repr: ethnum::I256::from_le_bytes(bytes),
        }
    }

    #[inline(always)]
    pub fn to_le_bytes(self) -> [u8; 32] {
        self.repr.to_le_bytes()
    }
}

impl From<ethnum::I256> for I256 {
    #[inline(always)]
    fn from(repr: ethnum::I256) -> Self {
        Self { repr }
    }
}

impl From<I256> for ethnum::I256 {
    #[inline(always)]
    fn from(value: I256) -> Self {
        value.repr
    }
}

impl From<I256> for BigInt {
    fn from(value: I256) -> Self {
        BigInt::from_signed_bytes_le(&value.to_le_bytes())
    }
}

impl std::fmt::Debug for I256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "I256({})", self.repr)
    }
}

// ---- Serialization

macro_rules! serde_serializer {
    ($wrapper:ty) => {
        impl Serialize for $wrapper {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.to_le_bytes().serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $wrapper {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Ok(<$wrapper>::from_le_bytes(
                    (<[u8; 32]>::deserialize(deserializer)?),
                ))
            }
        }
    };
}

serde_serializer!(U256);
serde_serializer!(I256);

// ---- Fuzzing

#[cfg(any(test, feature = "fuzzing"))]
mod arbitrary_impl {
    use super::*;
    use arbitrary::{Arbitrary, Result as AResult, Unstructured};
    use dearbitrary::{Dearbitrary, Dearbitrator};

    impl<'a> Arbitrary<'a> for U256 {
        fn arbitrary(u: &mut Unstructured<'a>) -> AResult<Self> {
            let bytes: [u8; 32] = <[u8; 32]>::arbitrary(u)?;
            Ok(U256::from_le_bytes(bytes))
        }
    }

    impl<'a> Arbitrary<'a> for I256 {
        fn arbitrary(u: &mut Unstructured<'a>) -> AResult<Self> {
            let bytes: [u8; 32] = <[u8; 32]>::arbitrary(u)?;
            Ok(I256::from_le_bytes(bytes))
        }
    }

    impl Dearbitrary for U256 {
        fn dearbitrary(&self, dearbitrator: &mut Dearbitrator) {
            dearbitrator.push_bytes(&self.to_le_bytes());
        }
    }

    impl Dearbitrary for I256 {
        fn dearbitrary(&self, dearbitrator: &mut Dearbitrator) {
            dearbitrator.push_bytes(&self.to_le_bytes());
        }
    }
}

// ---- Proptest

#[cfg(any(test, feature = "fuzzing"))]
mod proptest_impl {
    use super::*;
    use proptest::prelude::*;
    impl Arbitrary for U256 {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            any::<[u8; 32]>().prop_map(U256::from_le_bytes).boxed()
        }
    }

    impl Arbitrary for I256 {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            any::<[u8; 32]>().prop_map(I256::from_le_bytes).boxed()
        }
    }
}

// ---- String Representation

macro_rules! string_repr {
    ($wrapper:ty, $repr:ty) => {
        impl $wrapper {
            pub fn from_str_radix(s: &str, radix: u32) -> anyhow::Result<Self> {
                Ok(<$repr>::from_str_radix(s, radix)?.into())
            }
        }

        impl std::fmt::Display for $wrapper {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.repr)
            }
        }

        impl FromStr for $wrapper {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::from_str_radix(s, 10)
            }
        }
    };
}

string_repr!(U256, ethnum::U256);
string_repr!(I256, ethnum::I256);

// ---- Arithmetics

macro_rules! arithmetics {
    ($wrapper:ty, $repr:ty) => {
        impl $wrapper {
            pub fn checked_add(l: $wrapper, r: $wrapper) -> Option<$wrapper> {
                <$repr>::checked_add(l.repr, r.repr).map(|r| r.into())
            }

            pub fn checked_sub(l: $wrapper, r: $wrapper) -> Option<$wrapper> {
                <$repr>::checked_sub(l.repr, r.repr).map(|r| r.into())
            }

            pub fn checked_mul(l: $wrapper, r: $wrapper) -> Option<$wrapper> {
                <$repr>::checked_mul(l.repr, r.repr).map(|r| r.into())
            }

            pub fn checked_div(l: $wrapper, r: $wrapper) -> Option<$wrapper> {
                <$repr>::checked_div(l.repr, r.repr).map(|r| r.into())
            }

            pub fn checked_rem(l: $wrapper, r: $wrapper) -> Option<$wrapper> {
                <$repr>::checked_rem(l.repr, r.repr).map(|r| r.into())
            }
        }
    };
}

macro_rules! bitops {
    ($wrapper:ty, $repr:ty) => {
        impl BitAnd for $wrapper {
            type Output = $wrapper;

            fn bitand(self, rhs: Self) -> Self::Output {
                (self.repr & rhs.repr).into()
            }
        }

        impl BitOr for $wrapper {
            type Output = $wrapper;

            fn bitor(self, rhs: Self) -> Self::Output {
                (self.repr | rhs.repr).into()
            }
        }

        impl BitXor for $wrapper {
            type Output = $wrapper;

            fn bitxor(self, rhs: Self) -> Self::Output {
                (self.repr ^ rhs.repr).into()
            }
        }

        impl Shl for $wrapper {
            type Output = $wrapper;

            fn shl(self, rhs: Self) -> Self::Output {
                (self.repr << rhs.repr).into()
            }
        }

        impl Shr for $wrapper {
            type Output = $wrapper;

            fn shr(self, rhs: Self) -> Self::Output {
                (self.repr >> rhs.repr).into()
            }
        }
    };
}

arithmetics!(U256, ethnum::U256);
bitops!(U256, ethnum::U256);
arithmetics!(I256, ethnum::I256);

impl Neg for I256 {
    type Output = I256;

    fn neg(self) -> Self::Output {
        I256 { repr: -self.repr }
    }
}

impl I256 {
    pub fn checked_neg(self) -> Option<I256> {
        self.repr.checked_neg().map(|x| x.into())
    }
}

// ---- Conversions
// Semantics: conversions are fallible if the target type cannot represent the full range of the source type.

// Conversions between wrapper types and primitive types where both directions are fallible.
macro_rules! conversions {
    ($wrapper:ty, $repr:ty, $target:ty) => {
        impl TryFrom<$wrapper> for $target {
            type Error = anyhow::Error;

            #[inline(always)]
            fn try_from(value: $wrapper) -> Result<Self, Self::Error> {
                Ok(value.repr.try_into()?)
            }
        }

        impl TryFrom<$target> for $wrapper {
            type Error = anyhow::Error;

            #[inline(always)]
            fn try_from(value: $target) -> Result<Self, Self::Error> {
                let result: $repr = value.try_into()?;
                Ok(result.into())
            }
        }
    };
}

// Conversions where wrapper to primitive is fallible, but primitive to wrapper is infallible.
macro_rules! conversions_infallible {
    ($wrapper:ty, $repr:ty, $target:ty) => {
        impl TryFrom<$wrapper> for $target {
            type Error = anyhow::Error;

            #[inline(always)]
            fn try_from(value: $wrapper) -> Result<Self, Self::Error> {
                Ok(value.repr.try_into()?)
            }
        }

        impl From<$target> for $wrapper {
            fn from(value: $target) -> Self {
                let result: $repr = value.try_into().expect("conversion succeeds");
                result.into()
            }
        }
    };
}

conversions_infallible!(U256, ethnum::U256, u8);
conversions_infallible!(U256, ethnum::U256, u16);
conversions_infallible!(U256, ethnum::U256, u32);
conversions_infallible!(U256, ethnum::U256, u64);
conversions_infallible!(U256, ethnum::U256, u128);
conversions!(U256, ethnum::U256, i8);
conversions!(U256, ethnum::U256, i16);
conversions!(U256, ethnum::U256, i32);
conversions!(U256, ethnum::U256, i64);
conversions!(U256, ethnum::U256, i128);

conversions_infallible!(I256, ethnum::I256, u8);
conversions_infallible!(I256, ethnum::I256, u16);
conversions_infallible!(I256, ethnum::I256, u32);
conversions_infallible!(I256, ethnum::I256, u64);
conversions_infallible!(I256, ethnum::I256, u128);
conversions_infallible!(I256, ethnum::I256, i8);
conversions_infallible!(I256, ethnum::I256, i16);
conversions_infallible!(I256, ethnum::I256, i32);
conversions_infallible!(I256, ethnum::I256, i64);
conversions_infallible!(I256, ethnum::I256, i128);

// Conversion between those types is not the same as back and forth between primitives,
// therefore explicit trait impls.
impl TryFrom<I256> for U256 {
    type Error = anyhow::Error;

    fn try_from(value: I256) -> Result<Self, Self::Error> {
        let repr: ethnum::U256 = value.repr.try_into()?;
        Ok(U256 { repr })
    }
}

impl TryFrom<U256> for I256 {
    type Error = anyhow::Error;

    fn try_from(value: U256) -> Result<Self, Self::Error> {
        let repr: ethnum::I256 = value.repr.try_into()?;
        Ok(I256 { repr })
    }
}
