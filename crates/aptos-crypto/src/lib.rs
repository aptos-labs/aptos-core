// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! A library supplying various cryptographic primitives
pub mod bls12381;
pub mod compat;
pub mod ed25519;
pub mod error;
pub mod hash;
pub mod hkdf;
pub mod multi_ed25519;
pub mod noise;
pub mod test_utils;
pub mod traits;
pub mod validatable;
pub mod x25519;
pub mod pippenger;

#[cfg(test)]
mod unit_tests;

/// TBD.
pub fn msm_all_bench_cases() -> Vec<usize> {
    let series_until_65 = (1..65).step_by(2);
    let series_until_129 = (64..129).step_by(4);
    let series_until_257 = (129..257).step_by(8);
    series_until_65.chain(series_until_129).chain(series_until_257).collect::<Vec<_>>()
}

/// TBD.
#[macro_export]
macro_rules! rand {
    ($typ:ty) => {{
        <$typ>::rand(&mut test_rng())
    }}
}

/// TBD.
#[macro_export]
macro_rules! serialize {
    ($obj:expr, $method:ident) => {{
        let mut buf = vec![];
        $obj.$method(&mut buf).unwrap();
        buf
    }}
}


pub use self::traits::*;
pub use hash::HashValue;
// Reexport once_cell and serde_name for use in CryptoHasher Derive implementation.
#[doc(hidden)]
pub use once_cell as _once_cell;
#[doc(hidden)]
pub use serde_name as _serde_name;
