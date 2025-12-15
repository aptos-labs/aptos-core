// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    sigma_protocol,
    sigma_protocol::{
        homomorphism,
        homomorphism::{fixed_base_msms, fixed_base_msms::Trait, VectorShape},
    },
    Scalar,
};
use aptos_crypto::arkworks::msm::{IsMsmInput, MsmInput};
use aptos_crypto_derive::SigmaProtocolWitness;
use ark_ec::CurveGroup;
use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use std::fmt::Debug;

pub const DST: &[u8; 26] = b"APTOS_COMMIT_HOM_SIGMA_DST"; // This is used to create public parameters, see `default()` below

/// In this file we set up the following "commitment" homomorphism:
/// Commit to scalars by multiplying a base group element (in affine representation)
/// with each scalar.
///
/// Equivalent to `[base * s for s in scalars]`.
#[derive(CanonicalSerialize, Debug, Clone, PartialEq, Eq)]
pub struct Homomorphism<C: CurveGroup> {
    pub base: C::Affine,
}

pub type CodomainShape<T> = VectorShape<T>;

#[derive(
    SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq,
)]
pub struct Witness<F: PrimeField> {
    pub values: Vec<Scalar<F>>,
}

impl<C: CurveGroup> homomorphism::Trait for Homomorphism<C> {
    type Codomain = CodomainShape<C>;
    type Domain = Witness<C::ScalarField>;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        self.apply_msm(self.msm_terms(input))
    }
}

impl<C: CurveGroup> fixed_base_msms::Trait for Homomorphism<C> {
    type CodomainShape<T>
        = CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmInput = MsmInput<C::Affine, C::ScalarField>;
    type MsmOutput = C;
    type Scalar = C::ScalarField;

    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        // Create one MsmInput per scalar, each with its own cloned base.
        let inputs: Vec<_> = input
            .values
            .iter()
            .map(|scalar| MsmInput {
                bases: vec![self.base.clone()],
                scalars: vec![scalar.0],
            })
            .collect();

        VectorShape(inputs)
    }

    fn msm_eval(input: Self::MsmInput) -> Self::MsmOutput {
        input.bases()[0] * input.scalars()[0]
    }
}

impl<C: CurveGroup> sigma_protocol::Trait<C> for Homomorphism<C> {
    fn dst(&self) -> Vec<u8> {
        DST.to_vec()
    }
}