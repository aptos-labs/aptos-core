// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pcs::univariate_hiding_kzg,
    pvss::chunked_elgamal_field::chunked_elgamal,
    sigma_protocol,
    sigma_protocol::homomorphism::{tuple::TupleHomomorphism, LiftHomomorphism},
    Scalar,
};
use aptos_crypto_derive::SigmaProtocolWitness;
use ark_ec::pairing::Pairing;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::rand::{CryptoRng, RngCore};
use ark_ec::AdditiveGroup;

#[derive(SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct HkzgElgamalWitness<E: Pairing> {
    pub hkzg_randomness: Scalar<E>,
    pub chunked_plaintexts: Vec<Vec<Scalar<E>>>,
    pub elgamal_randomness: Vec<Scalar<E>>,
}

type LiftedKZG<'a, E> =
    LiftHomomorphism<univariate_hiding_kzg::CommitmentHomomorphism<'a, E>, HkzgElgamalWitness<E>>;
type LiftedChunkedElGamal<'a, E> =
    LiftHomomorphism<chunked_elgamal::Homomorphism<'a, E>, HkzgElgamalWitness<E>>;

pub type HkzgChunkedElgamalHomomorphism<'a, E> =
    TupleHomomorphism<LiftedKZG<'a, E>, LiftedChunkedElGamal<'a, E>>;

impl<'a, E: Pairing> HkzgChunkedElgamalHomomorphism<'a, E> {
    pub fn new(
        lagr_g1: &'a [E::G1Affine],
        xi_1: E::G1Affine,
        g_1: &'a E::G1Affine,
        h_1: &'a E::G1Affine,
        ek: &'a [E::G1Affine],
    ) -> Self {
        let lifted_kzg = LiftedKZG::<E> {
            hom: univariate_hiding_kzg::CommitmentHomomorphism { lagr_g1, xi_1 },
            projection: |dom: &HkzgElgamalWitness<E>| {
                let HkzgElgamalWitness {
                    hkzg_randomness: kzg_randomness,
                    chunked_plaintexts,
                    ..
                } = dom;
                let flattened: Vec<E::ScalarField> = {
                    let scalars: Vec<Scalar<E>> = std::iter::once(Scalar(E::ScalarField::ZERO))
                        .chain(chunked_plaintexts.iter().flatten().cloned())
                        .collect();
                    Scalar::<E>::vec_into_inner(scalars)
                };
                (kzg_randomness.0, flattened)
            },
        };
        let lifted_chunked_elgamal = LiftedChunkedElGamal::<E> {
            hom: chunked_elgamal::Homomorphism { g_1, h_1, ek },
            projection: |dom: &HkzgElgamalWitness<E>| {
                let HkzgElgamalWitness {
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