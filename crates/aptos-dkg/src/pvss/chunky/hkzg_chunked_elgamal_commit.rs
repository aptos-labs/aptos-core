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
            tuple::{PairingTupleHomomorphism, TupleCodomainShape},
            LiftHomomorphism,
        },
        traits::FirstProofItem,
    },
};
use aptos_crypto::{
    arkworks::random::unsafe_random_points_group, weighted_config::WeightedConfigArkworks,
    TSecretSharingConfig,
};
use ark_ec::{pairing::Pairing, AffineRepr, CurveGroup};

type HkzgElgamalHomomorphism<'a, E> = hkzg_chunked_elgamal::WeightedHomomorphism<'a, E>;
type LiftedCommitHomomorphism<C> = LiftHomomorphism<
    chunked_scalar_mul::Homomorphism<C>,
    HkzgWeightedElgamalWitness<<<C as CurveGroup>::Affine as AffineRepr>::ScalarField>,
>;

pub type Homomorphism<'a, E> = PairingTupleHomomorphism<
    E,
    HkzgElgamalHomomorphism<'a, E>,
    LiftedCommitHomomorphism<<E as Pairing>::G2>,
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
            FirstProofItem::Commitment(first_proof_item_inner) => {
                Self {
                    first_proof_item: FirstProofItem::Commitment(TupleCodomainShape(
                        first_proof_item_inner,
                        chunked_scalar_mul::CodomainShape::<E::G2>(
                            (0..sc.get_total_num_players()) // TODO: make this stuff less complicated!!!
                                .map(|i| {
                                    let w = sc.get_player_weight(&sc.get_player(i)); // TODO: combine these functions...
                                    unsafe_random_points_group(w, rng)
                                })
                                .collect(),
                        ),
                    )),
                    z,
                }
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
        eks: &'a [E::G1Affine],
        base: E::G2Affine,
        ell: u8,
    ) -> Self {
        // Set up the HKZG-EG homomorphism, and use a projection map to lift it to HkzgElgamalCommitWitness
        let hkzg_el_hom =
            hkzg_chunked_elgamal::WeightedHomomorphism::<E>::new(lagr_g1, xi_1, pp, eks);

        // Set up the lifted commit homomorphism
        let lifted_commit_hom = LiftedCommitHomomorphism::<E::G2> {
            hom: chunked_scalar_mul::Homomorphism { base, ell },
            // The projection map simply unchunks the chunks
            projection: |dom: &HkzgWeightedElgamalWitness<E::ScalarField>| {
                chunked_scalar_mul::Witness {
                    chunked_values: dom.chunked_plaintexts.clone(),
                }
            },
        };

        // Combine the two lifted homomorphisms just constructed, into the required `TupleHomomorphism`
        Self {
            hom1: hkzg_el_hom,
            hom2: lifted_commit_hom,
            _pairing: std::marker::PhantomData,
        }
    }
}
