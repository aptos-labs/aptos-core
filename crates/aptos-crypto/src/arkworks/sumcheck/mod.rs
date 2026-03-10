// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//
//! Sumcheck protocol and Product-of-4-MLEs instance.
//! Adapted from Jolt (https://github.com/a16z/jolt) for use in aptos-crypto without pulling in jolt-core.

#![allow(missing_docs)]
#![allow(non_snake_case)]

pub mod booleanity_eq;
pub mod booleanity_eq_lsb;
mod dense_poly;
mod field;
mod gaussian_elimination;
mod masking;
mod merlin_transcript;
mod opening;
mod protocol;
mod transcript;
mod unipoly;

pub mod product4;
pub mod traits;

pub use booleanity_eq::{
    BooleanityEqParams, BooleanityEqSumcheckProver, BooleanityEqSumcheckVerifier,
};
pub use booleanity_eq_lsb::{
    BooleanityEqLsbParams, BooleanityEqSumcheckProverLSB, BooleanityEqSumcheckVerifierLSB,
    BooleanityEqSumcheckVerifierLSBWithOpenings,
};
pub use dense_poly::{BindingOrder, DensePolynomial};
pub use field::SumcheckField;
pub use masking::{expand_seed_to_field, MaskingPolynomial};
pub use merlin_transcript::MerlinSumcheckTranscript;
pub use opening::{ProverOpeningAccumulator, VerifierOpeningAccumulator};
pub use product4::{Product4SumcheckProver, Product4SumcheckVerifier};
pub use protocol::{BatchedSumcheck, ClearSumcheckProof};
pub use traits::{
    OpeningAccumulator, SumcheckInstanceParams, SumcheckInstanceProver, SumcheckInstanceVerifier,
};
pub use transcript::SumcheckTranscript;
pub use unipoly::{CompressedUniPoly, UniPoly};
