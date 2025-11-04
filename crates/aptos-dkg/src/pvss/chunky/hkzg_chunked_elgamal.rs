// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pcs::univariate_hiding_kzg,
    pvss::chunky::chunked_elgamal,
    sigma_protocol,
    sigma_protocol::homomorphism::{tuple::TupleHomomorphism, LiftHomomorphism},
    Scalar,
};
use aptos_crypto_derive::SigmaProtocolWitness;
use ark_ec::{pairing::Pairing, AdditiveGroup};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::rand::{CryptoRng, RngCore};

/// Witness data for the `chunked_elgamal_field` PVSS protocol.
///
/// In this PVSS scheme, plaintexts (which are shares) are first divided into chunks. Then
/// two more (independent) pieces of data are generated:
///
/// - **HKZG randomness** is generated and used in the DeKARTv2 range proof,
///    to prove that the chunks lie in the correct range.
/// - **ElGamal randomness** is generated and used to encrypt the chunks.
///
/// To prove consistency between these components, we thus construct a Σ-protocol
/// defined over a domain that jointly includes:
/// - the HKZG randomness,
/// - the chunked plaintexts, and
/// - the ElGamal randomness.
#[derive(
    SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Debug, Clone, PartialEq, Eq,
)]
pub struct HkzgElgamalWitness<E: Pairing> {
    pub hkzg_randomness: Scalar<E>,
    pub chunked_plaintexts: Vec<Vec<Scalar<E>>>,
    pub elgamal_randomness: Vec<Scalar<E>>,
}

/// The two components described earlier — (1) generating HKZG randomness for the DeKARTv2 proof
/// and (2) encrypting with ElGamal randomness — are part of a single Σ-protocol
/// proving knowledge of a *preimage* under a tuple homomorphism, consisting of:
///
/// (i) the HKZG commitment homomorphism, and
/// (ii) the `chunked_elgamal` homomorphism.
///
/// On the domain side, each of the two parts of this tuple homomorphism corresponds to one of the
/// two components: in each case, the witness omits (or “ignores”) one of its three fields, then applies
/// a homomorphism. Thus, the overall homomorphism of the Σ-protocol can be viewed as a tuple of two
/// *lifted* homomorphisms.
type LiftedHkzg<'a, E> =
    LiftHomomorphism<univariate_hiding_kzg::CommitmentHomomorphism<'a, E>, HkzgElgamalWitness<E>>;
type LiftedChunkedElgamal<'a, E> =
    LiftHomomorphism<chunked_elgamal::Homomorphism<'a, E>, HkzgElgamalWitness<E>>;

pub type Homomorphism<'a, E> = TupleHomomorphism<LiftedHkzg<'a, E>, LiftedChunkedElgamal<'a, E>>;

#[allow(non_snake_case)]
impl<'a, E: Pairing> Homomorphism<'a, E> {
    pub fn new(
        lagr_g1: &'a [E::G1Affine], // TODO: could combine lagr_g1 and xi_1 into hkzg::PublicParamaters<E>?
        xi_1: E::G1Affine,
        pp: &'a chunked_elgamal::PublicParameters<E>,
        eks: &'a [E::G1Affine],
    ) -> Self {
        // Set up the HKZG homomorphism, and use a projection map to lift it to HkzgElgamalWitness
        let lifted_hkzg = LiftedHkzg::<E> {
            hom: univariate_hiding_kzg::CommitmentHomomorphism { lagr_g1, xi_1 },
            // The projection map ignores the `elgamal_randomness` component, and flattens the vector of chunked plaintexts
            projection: |dom: &HkzgElgamalWitness<E>| {
                let HkzgElgamalWitness {
                    hkzg_randomness,
                    chunked_plaintexts,
                    ..
                } = dom;
                let flattened_chunked_plaintexts: Vec<Scalar<E>> =
                    std::iter::once(Scalar(E::ScalarField::ZERO))
                        .chain(chunked_plaintexts.iter().flatten().cloned())
                        .collect();
                univariate_hiding_kzg::Witness::<E> {
                    hiding_randomness: Scalar(hkzg_randomness.0),
                    values: flattened_chunked_plaintexts,
                }
            },
        };
        // Set up the chunked_elgamal homomorphism, and use a projection map to lift it to HkzgElgamalWitness
        let lifted_chunked_elgamal = LiftedChunkedElgamal::<E> {
            hom: chunked_elgamal::Homomorphism { pp, eks },
            // The projection map simply ignores the `hkzg_randomness` component
            projection: |dom: &HkzgElgamalWitness<E>| {
                let HkzgElgamalWitness {
                    chunked_plaintexts,
                    elgamal_randomness,
                    ..
                } = dom;
                chunked_elgamal::Witness {
                    plaintext_chunks: chunked_plaintexts.clone(),
                    plaintext_randomness: elgamal_randomness.clone(),
                }
            },
        };

        // Combine the two lifted homomorphisms, into the required TupleHomomorphism
        Self {
            hom1: lifted_hkzg,
            hom2: lifted_chunked_elgamal,
        }
    }
}
