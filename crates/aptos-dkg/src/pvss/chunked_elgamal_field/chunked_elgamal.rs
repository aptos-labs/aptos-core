// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    sigma_protocol::homomorphism::{
        self, fixedbasemsms, fixedbasemsms::FixedBaseMsms, EntrywiseMap,
    },
    Scalar,
};
use ark_ec::{pairing::Pairing, VariableBaseMSM};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

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
    pub randomness: Vec<T>,
}
type MatrixVectorPair<T> = (Vec<Vec<T>>, Vec<T>); // Domain shape happens to be similar to Codomain shape

impl<'a, E: Pairing> homomorphism::Trait for Homomorphism<'a, E> {
    type Codomain = CodomainShape<E::G1>;
    type Domain = MatrixVectorPair<Scalar<E>>;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        self.apply_msm(self.msm_terms(input))
    }
}

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
impl<'a, E: Pairing> FixedBaseMsms for Homomorphism<'a, E> {
    type Base = Base<E>;
    type CodomainShape<T>
        = CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone;
    type MsmInput = fixedbasemsms::MsmInput<Self::Base, Self::Scalar>;
    type MsmOutput = E::G1;
    type Scalar = Scalar<E>;

    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        let Cs = input
            .0
            .iter()
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .zip(input.1.iter())
                    .map(|(&z_ij, &r_j)| fixedbasemsms::MsmInput {
                        bases: vec![*self.g_1, self.ek[i]],
                        scalars: vec![z_ij, r_j],
                    })
                    .collect()
            })
            .collect();

        let Rs = input
            .1
            .iter()
            .map(|&r_j| fixedbasemsms::MsmInput {
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
