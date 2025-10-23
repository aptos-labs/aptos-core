// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pcs::univariate_kzg,
    pvss::chunked_elgamal_field::chunked_elgamal,
    sigma_protocol,
    sigma_protocol::homomorphism::{tuple::TupleHomomorphism, LiftHomomorphism},
    Scalar,
};
use aptos_crypto_derive::SigmaProtocolWitness;
use ark_ec::pairing::Pairing;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::rand::{CryptoRng, RngCore};

#[derive(SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Debug, Clone)]
pub struct KzgElgamalWitness<E: Pairing> {
    pub kzg_randomness: Scalar<E>,
    pub chunked_plaintexts: Vec<Vec<Scalar<E>>>,
    pub elgamal_randomness: Vec<Scalar<E>>,
}

#[allow(type_alias_bounds)]
type LiftedKZG<'a, E: Pairing> =
    LiftHomomorphism<univariate_kzg::Homomorphism<'a, E>, KzgElgamalWitness<E>>;
#[allow(type_alias_bounds)]
type LiftedChunkedElGamal<'a, E: Pairing> =
    LiftHomomorphism<chunked_elgamal::Homomorphism<'a, E>, KzgElgamalWitness<E>>;

pub type KzgChunkedElgamalHomomorphism<'a, E> =
    TupleHomomorphism<LiftedKZG<'a, E>, LiftedChunkedElGamal<'a, E>>;

impl<'a, E: Pairing> KzgChunkedElgamalHomomorphism<'a, E> {
    pub fn new(
        lagr_g1: &'a [E::G1Affine],
        g_1: &'a E::G1Affine,
        h_1: &'a E::G1Affine,
        ek: &'a [E::G1Affine],
    ) -> Self {
        let lifted_kzg = LiftedKZG::<E> {
            hom: univariate_kzg::Homomorphism { lagr_g1 },
            projection: |dom: &KzgElgamalWitness<E>| {
                let KzgElgamalWitness {
                    kzg_randomness,
                    chunked_plaintexts,
                    ..
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
                    chunked_plaintexts,
                    elgamal_randomness,
                    ..
                } = dom;
                chunked_elgamal::Witness{ chunks: chunked_plaintexts.clone(), randomness: elgamal_randomness.clone() }
            },
        };

        Self {
            hom1: lifted_kzg,
            hom2: lifted_chunked_elgamal,
            dst: b"Kzg-Elgamal tuple DST".to_vec(),
            dst_verifier: b"Kzg-Elgamal tuple verifier DST".to_vec(),
        }
    }
}