#![forbid(unsafe_code)]
//#![cfg_attr(not(feature = "std"), no_std)]
//! A crate for sumcheck protocol of GKR functions
#![deny(unused_import_braces, unused_qualifications, trivial_casts)]
#![deny(trivial_numeric_casts, variant_size_differences)]
#![deny(stable_features, unreachable_pub, non_shorthand_field_patterns)]
#![deny(unused_attributes)]
#![deny(renamed_and_removed_lints, stable_features, unused_allocation)]
#![deny(unused_comparisons, bare_trait_objects, unused_must_use)]

pub use error::Error;

// /// use ark_std for std
// #[macro_use]
// extern crate ark_std;

/// error for this crate
mod error;

pub mod ml_sumcheck;

pub mod rng;

#[cfg(test)]
mod tests {}
