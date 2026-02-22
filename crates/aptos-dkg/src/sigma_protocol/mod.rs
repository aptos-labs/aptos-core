// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Σ-protocols (sigma protocols) for proving knowledge of preimages under group homomorphisms.
//! Used in PVSS, range proofs, and PCS components; supports Fiat–Shamir and batched verification.

pub mod homomorphism;
pub mod proof;
pub mod traits;

pub use proof::{FirstProofItem, Proof};
pub use traits::{
    check_msm_eval_zero, verifier_challenges_with_length, CurveGroupTrait, Statement, Trait,
    Witness,
};
