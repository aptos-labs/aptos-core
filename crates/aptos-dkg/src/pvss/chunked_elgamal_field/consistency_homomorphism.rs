// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    algebra::morphism::{DiagonalProductMorphism, LiftMorphism},
    pcs::univariate_kzg::UnivariateKZG,
    pvss::chunked_elgamal_field::chunked_elgamal::ChunkedElGamal,
    sigma_protocol,
};
use ark_ec::pairing::Pairing;
use ark_serialize::CanonicalSerialize;
use ark_std::{
    rand::{CryptoRng, RngCore},
    UniformRand,
};

#[derive(CanonicalSerialize, Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyDomain<E: Pairing>(
    pub E::ScalarField,
    pub Vec<Vec<E::ScalarField>>,
    pub Vec<E::ScalarField>,
);

impl<E: Pairing> sigma_protocol::Domain<E> for ConsistencyDomain<E> {
    type Scalar = E::ScalarField;

    fn scaled_add(&self, other: &Self, c: E::ScalarField) -> Self {
        ConsistencyDomain(
            self.0 + (c * other.0),
            self.1
                .iter()
                .zip(&other.1)
                .map(|(r1, r2)| r1.iter().zip(r2).map(|(x, y)| *x + (c * *y)).collect())
                .collect(),
            self.2
                .iter()
                .zip(&other.2)
                .map(|(x, y)| *x + (c * *y))
                .collect(),
        )
    }

    fn sample_randomness<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self {
        ConsistencyDomain(
            E::ScalarField::rand(rng),
            self.1
                .iter()
                .map(|row| row.iter().map(|_| E::ScalarField::rand(rng)).collect())
                .collect(),
            self.2.iter().map(|_| E::ScalarField::rand(rng)).collect(),
        )
    }
}

#[allow(type_alias_bounds)]
type LiftedKZG<'a, E: Pairing> = LiftMorphism<UnivariateKZG<'a, E>, ConsistencyDomain<E>>;
#[allow(type_alias_bounds)]
type LiftedChunkedElGamal<'a, E: Pairing> =
    LiftMorphism<ChunkedElGamal<'a, E>, ConsistencyDomain<E>>;

pub type ConsistencyHomomorphism<'a, E> =
    DiagonalProductMorphism<LiftedKZG<'a, E>, LiftedChunkedElGamal<'a, E>>;

impl<'a, E: Pairing> ConsistencyHomomorphism<'a, E> {
    pub fn new(
        lagr_g1: &'a [E::G1Affine],
        g_1: &'a E::G1Affine,
        h_1: &'a E::G1Affine,
        ek: &'a [E::G1Affine],
    ) -> Self {
        let lifted_kzg = LiftedKZG::<E> {
            morphism: UnivariateKZG { lagr_g1 },
            projection_map: |dom: &ConsistencyDomain<E>| {
                let ConsistencyDomain(first, nested, _ignored) = dom;
                let flattened: Vec<E::ScalarField> = nested.iter().flatten().cloned().collect();
                (first.clone(), flattened)
            },
        };

        let lifted_chunked_elgamal = LiftedChunkedElGamal::<E> {
            morphism: ChunkedElGamal { g_1, h_1, ek },
            projection_map: |dom: &ConsistencyDomain<E>| {
                let ConsistencyDomain(_ignored, first, second) = dom;
                (first.clone(), second.clone())
            },
        };

        Self {
            morphism1: lifted_kzg,
            morphism2: lifted_chunked_elgamal,
        }
    }
}
