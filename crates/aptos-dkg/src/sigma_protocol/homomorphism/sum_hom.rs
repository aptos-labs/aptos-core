// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// ===============================================================================
// Sum homomorphism: domain = vector of field elements, codomain = field (sum).
// ===============================================================================

use crate::sigma_protocol::homomorphism;
use ark_ff::PrimeField;
use ark_serialize::CanonicalSerialize;
use std::marker::PhantomData;

/// Homomorphism that maps a vector of prime field elements to their sum.
/// Domain = `Vec<F>`, Codomain = CodomainNormalized = `F`.
#[derive(CanonicalSerialize, Clone, Debug, PartialEq, Eq)]
pub struct SumHomomorphism<F: PrimeField>(PhantomData<F>);

impl<F: PrimeField> Default for SumHomomorphism<F> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<F: PrimeField> homomorphism::Trait for SumHomomorphism<F> {
    type Codomain = F;
    type CodomainNormalized = F;
    type Domain = Vec<F>;

    fn apply(&self, element: &Self::Domain) -> Self::Codomain {
        element.iter().fold(F::zero(), |acc, x| acc + x)
    }

    fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
        value
    }
}
