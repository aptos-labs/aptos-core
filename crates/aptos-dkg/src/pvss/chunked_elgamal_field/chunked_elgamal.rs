// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    sigma_protocol::homomorphism::{self, fixed_base_msms, fixed_base_msms::Trait, EntrywiseMap},
    Scalar,
};
use ark_ec::{pairing::Pairing, VariableBaseMSM};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::fmt::Debug;

type Base<E> = <E as Pairing>::G1Affine;

/// Formally, given:
/// - `g_1, h_1` ∈ G₁ (group generators)
/// - `ek_i` ∈ G₁ (encryption keys)
/// - `z_ij` ∈ Scalar<E> (input plaintext chunk)
/// - `r_j` ∈ Scalar<E> (randomness for each `column` of chunks z_ij)
///
/// The homomorphism maps input `Z = [z_ij]` and randomness `R = [r_j]` to
/// codomain elements:
///
/// ```text
/// C_ij = g_1 * z_ij + ek_i * r_j
/// R_j  = h_1 * r_j
/// ```
///
/// The `C_ij` represent "chunked" homomorphic encryptions of the plaintexts,
/// and `R_j` carry the corresponding randomness contributions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct Homomorphism<'a, E: Pairing> {
    pub g_1: &'a Base<E>,
    pub h_1: &'a Base<E>,
    pub eks: &'a [Base<E>],
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
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
impl<T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq> EntrywiseMap<T>
    for CodomainShape<T>
{
    type Output<U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq> =
        CodomainShape<U>;

    fn map<U, F>(self, f: F) -> Self::Output<U>
    where
        F: Fn(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq,
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
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmInput = fixed_base_msms::MsmInput<Self::Base, Self::Scalar>;
    type MsmOutput = E::G1;
    type Scalar = Scalar<E>;

    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        // C_ij = g_1 * z_ij + ek[i] * r_j
        let Cs = input
            .chunks
            .iter()
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .zip(input.randomness.iter())
                    .map(|(&z_ij, &r_j)| fixed_base_msms::MsmInput {
                        bases: vec![*self.g_1, self.eks[i]],
                        scalars: vec![z_ij, r_j],
                    })
                    .collect()
            })
            .collect();

        //  R_j = h_1 * r_j
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
        E::G1::msm(bases, &Scalar::slice_as_inner(scalars)).expect("MSM failed in ChunkedElGamal")
    }
}
