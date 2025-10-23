// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    sigma_protocol::homomorphism::{
        self, fixed_base_msms, fixed_base_msms::Trait, EntrywiseMap,
    },
    Scalar,
};
use ark_ec::{pairing::Pairing, VariableBaseMSM};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
// use crate::sigma_protocol;
// use aptos_crypto_derive::SigmaProtocolWitness;
// use ark_std::rand::{RngCore, CryptoRng};

type Base<E> = <E as Pairing>::G1Affine;

#[allow(non_snake_case)]
pub struct Homomorphism<'a, E: Pairing> {
    pub g_1: &'a Base<E>,
    pub h_1: &'a Base<E>,
    pub ek: &'a [Base<E>],
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone)]
pub struct CodomainShape<T: CanonicalSerialize + CanonicalDeserialize + Clone> {
    pub chunks: Vec<Vec<T>>, // Depending on T, these can be chunked plaintexts, chunked ciphertexts, their MSM representations, etc
    pub randomness: Vec<T>,  // Same story, depending on T
}

// Witness shape happens to be identical to CodomainShape, this is mostly coincidental
pub type Witness<E> = CodomainShape<Scalar<E>>;

impl<E: Pairing> homomorphism::Trait for Homomorphism<'_, E> {
    type Codomain = CodomainShape<E::G1>;
    type Domain = Witness<E>;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        self.apply_msm(self.msm_terms(input))
    }
}

// TODO: Can problably do EntrywiseMap with another derive macro
impl<T: CanonicalSerialize + CanonicalDeserialize + Clone> EntrywiseMap<T> for CodomainShape<T> {
    type Output<U: CanonicalSerialize + CanonicalDeserialize + Clone> = CodomainShape<U>;

    fn map<U, F>(self, f: F) -> Self::Output<U>
    where
        F: Fn(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone,
    {
        let chunks = self
            .chunks
            .into_iter()
            .map(|row| row.into_iter().map(&f).collect())
            .collect();

        let randomness = self.randomness.into_iter().map(f).collect();

        CodomainShape { chunks, randomness }
    }
}

// TODO: Use a derive macro?
impl<T: CanonicalSerialize + CanonicalDeserialize + Clone> IntoIterator for CodomainShape<T> {
    type IntoIter = std::vec::IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        let mut combined: Vec<T> = self.chunks.into_iter().flatten().collect(); // Temporary Vec can probably be avoided, but might require unstable Rust or a lot of lines
        combined.extend(self.randomness);
        combined.into_iter()
    }
}

#[allow(non_snake_case)]
impl<'a, E: Pairing> fixed_base_msms::Trait for Homomorphism<'a, E> {
    type Base = Base<E>;
    type CodomainShape<T>
        = CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone;
    type MsmInput = fixed_base_msms::MsmInput<Self::Base, Self::Scalar>;
    type MsmOutput = E::G1;
    type Scalar = Scalar<E>;

    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        let Cs = input
            .chunks
            .iter()
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .zip(input.randomness.iter())
                    .map(|(&z_ij, &r_j)| fixed_base_msms::MsmInput {
                        bases: vec![*self.g_1, self.ek[i]],
                        scalars: vec![z_ij, r_j],
                    })
                    .collect()
            })
            .collect();

        let Rs = input
            .randomness
            .iter()
            .map(|&r_j| fixed_base_msms::MsmInput {
                bases: vec![*self.h_1],
                scalars: vec![r_j],
            })
            .collect();

        CodomainShape {
            chunks: Cs,
            randomness: Rs,
        }
    }

    fn msm_eval(bases: &[Self::Base], scalars: &[Self::Scalar]) -> Self::MsmOutput {
        E::G1::msm(bases, Scalar::slice_as_inner(scalars)).expect("MSM failed in ChunkedElGamal")
    }
}