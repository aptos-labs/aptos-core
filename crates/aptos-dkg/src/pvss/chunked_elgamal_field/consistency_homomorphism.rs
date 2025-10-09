// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pcs::univariate_kzg_commitment,
    pvss::chunked_elgamal_field::chunked_elgamal,
    sigma_protocol,
    sigma_protocol::homomorphism::{LiftHomomorphism, TupleHomomorphism},
    Scalar,
};
use aptos_crypto_derive::Witness;
use ark_ec::pairing::Pairing;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::rand::{CryptoRng, RngCore};

#[derive(Witness, CanonicalSerialize, CanonicalDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct KzgElgamalWitness<E: Pairing> {
    pub kzg_randomness: Scalar<E>,
    pub chunked_plaintexts: Vec<Vec<Scalar<E>>>,
    pub elgamal_randomness: Vec<Scalar<E>>,
}

// impl<E: Pairing> sigma_protocol::Witness<E> for KzgElgamalWitness<E> {
//     type Scalar = E::ScalarField;

//     fn scaled_add(self, other: &Self, c: E::ScalarField) -> Self {
//         Self {
//             kzg_randomness: self.kzg_randomness.scaled_add(&other.kzg_randomness, c),
//             chunked_plaintexts: self.chunked_plaintexts.scaled_add(&other.chunked_plaintexts, c),
//             elgamal_randomness: self.elgamal_randomness.scaled_add(&other.elgamal_randomness, c),
//         }
//     }

//     fn rand<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self {
//         Self {
//             kzg_randomness: self.kzg_randomness.rand(rng),
//             chunked_plaintexts: self.chunked_plaintexts.rand(rng),
//             elgamal_randomness: self.elgamal_randomness.rand(rng),
//         }
//     }
// }

#[allow(type_alias_bounds)]
type LiftedKZG<'a, E: Pairing> =
    LiftHomomorphism<univariate_kzg_commitment::Homomorphism<'a, E>, KzgElgamalWitness<E>>;
#[allow(type_alias_bounds)]
type LiftedChunkedElGamal<'a, E: Pairing> =
    LiftHomomorphism<chunked_elgamal::Homomorphism<'a, E>, KzgElgamalWitness<E>>;

pub type KzgElgamalHomomorphism<'a, E> =
    TupleHomomorphism<LiftedKZG<'a, E>, LiftedChunkedElGamal<'a, E>>;

impl<'a, E: Pairing> KzgElgamalHomomorphism<'a, E> {
    pub fn new(
        lagr_g1: &'a [E::G1Affine],
        g_1: &'a E::G1Affine,
        h_1: &'a E::G1Affine,
        ek: &'a [E::G1Affine],
    ) -> Self {
        let lifted_kzg = LiftedKZG::<E> {
            hom: univariate_kzg_commitment::Homomorphism { lagr_g1 },
            projection: |dom: &KzgElgamalWitness<E>| {
                let KzgElgamalWitness {
                    kzg_randomness,
                    chunked_plaintexts,
                    elgamal_randomness: _,
                } = dom;
                let flattened: Vec<E::ScalarField> = chunked_plaintexts
                    .iter()
                    .flatten()
                    .map(|scalar| &scalar.0)
                    .cloned()
                    .collect();
                (kzg_randomness.0, flattened)
            },
        };

        let lifted_chunked_elgamal = LiftedChunkedElGamal::<E> {
            hom: chunked_elgamal::Homomorphism { g_1, h_1, ek },
            projection: |dom: &KzgElgamalWitness<E>| {
                let KzgElgamalWitness {
                    kzg_randomness: _,
                    chunked_plaintexts,
                    elgamal_randomness,
                } = dom;
                (chunked_plaintexts.clone(), elgamal_randomness.clone())
            },
        };

        Self {
            hom1: lifted_kzg,
            hom2: lifted_chunked_elgamal,
        }
    }
}
