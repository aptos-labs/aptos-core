// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    pvss::chunky::{
        chunked_elgamal_pp, chunked_scalar_mul, hkzg_chunked_elgamal,
        hkzg_chunked_elgamal::HkzgWeightedElgamalWitness,
    },
    sigma_protocol,
    sigma_protocol::{
        homomorphism::{
            tuple::{TupleCodomainShape, TupleHomomorphism},
            LiftHomomorphism,
        },
        FirstProofItem,
    },
};
use aptos_crypto::{
    arkworks::random::unsafe_random_points, weighted_config::WeightedConfigArkworks,
};
use ark_ec::{pairing::Pairing, scalar_mul::BatchMulPreprocessing, AffineRepr, CurveGroup};

pub(crate) type HkzgElgamalHomomorphism<'a, E> = hkzg_chunked_elgamal::WeightedHomomorphism<'a, E>;
pub(crate) type LiftedCommitHomomorphism<'a, C> = LiftHomomorphism<
    chunked_scalar_mul::Homomorphism<'a, C>,
    HkzgWeightedElgamalWitness<<<C as CurveGroup>::Affine as AffineRepr>::ScalarField>,
>;

pub type Homomorphism<'a, E> = TupleHomomorphism<
    HkzgElgamalHomomorphism<'a, E>,
    LiftedCommitHomomorphism<'a, <E as Pairing>::G2>,
>;
pub type Proof<'a, E> = sigma_protocol::Proof<<E as Pairing>::ScalarField, Homomorphism<'a, E>>;

impl<'a, E: Pairing> Proof<'a, E> {
    /// Generates a random looking proof (but not a valid one).
    /// Useful for testing and benchmarking.
    pub fn generate<R: rand::Rng + rand::CryptoRng>(
        sc: &WeightedConfigArkworks<E::ScalarField>,
        number_of_chunks_per_share: usize,
        rng: &mut R,
    ) -> Self {
        // or should number_of_chunks_per_share be a const?
        let hkzg_chunked_elgamal::WeightedProof::<E> {
            first_proof_item,
            z,
        } = hkzg_chunked_elgamal::WeightedProof::generate(sc, number_of_chunks_per_share, rng);
        match first_proof_item {
            FirstProofItem::Commitment(first_proof_item_inner) => Self {
                first_proof_item: FirstProofItem::Commitment(TupleCodomainShape(
                    first_proof_item_inner,
                    chunked_scalar_mul::CodomainShape(unsafe_random_points::<E::G2, _>(
                        sc.get_total_weight(),
                        rng,
                    )),
                )),
                z,
            },
            FirstProofItem::Challenge(_) => {
                panic!("Unexpected Challenge variant!");
            },
        }
    }
}

#[allow(non_snake_case)]
impl<'a, E: Pairing> Homomorphism<'a, E> {
    pub fn new(
        lagr_g1: &'a [E::G1Affine],
        xi_1: E::G1Affine,
        pp: &'a chunked_elgamal_pp::PublicParameters<E::G1>,
        G2_table: &'a BatchMulPreprocessing<E::G2>,
        eks: &'a [E::G1Affine],
        base: E::G2Affine,
        ell: u8,
    ) -> Self {
        // Set up the HKZG-EG homomorphism, and use a projection map to lift it to HkzgElgamalCommitWitness
        let hkzg_el_hom =
            hkzg_chunked_elgamal::WeightedHomomorphism::<E>::new(lagr_g1, xi_1, pp, eks);

        // Set up the lifted commit homomorphism
        let lifted_commit_hom = LiftedCommitHomomorphism::<'a, E::G2> {
            hom: chunked_scalar_mul::Homomorphism {
                base,
                table: G2_table,
                ell,
            },
            // The projection map simply unchunks the chunks
            projection: |dom: &HkzgWeightedElgamalWitness<E::ScalarField>| {
                chunked_scalar_mul::Witness {
                    chunked_values: dom.chunked_plaintexts.iter().flatten().cloned().collect(),
                }
            },
        };

        // Combine the two lifted homomorphisms just constructed, into the required `TupleHomomorphism`
        Self {
            hom1: hkzg_el_hom,
            hom2: lifted_commit_hom,
        }
    }
}
