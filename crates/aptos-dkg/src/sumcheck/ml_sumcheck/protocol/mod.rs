// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Interactive Proof Protocol used for Multilinear Sumcheck

use ark_ff::Field;
use ark_std::marker::PhantomData;

pub mod prover;
pub mod verifier;
pub use crate::sumcheck::ml_sumcheck::data_structures::{
    ListOfProductsOfPolynomials, PolynomialInfo,
};
/// Interactive Proof for Multilinear Sumcheck
pub struct IPForMLSumcheck<F: Field> {
    #[doc(hidden)]
    _marker: PhantomData<F>,
}
