// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    pcs::univariate_hiding_kzg,
    pvss::chunky::{
        chunked_elgamal, hkzg_chunked_elgamal, hkzg_chunked_elgamal::HkzgWeightedElgamalWitness,
        scalar_mul,
    },
    sigma_protocol,
    sigma_protocol::homomorphism::{tuple::PairingTupleHomomorphism, LiftHomomorphism},
    Scalar,
};
use aptos_crypto_derive::SigmaProtocolWitness;
use ark_ec::{pairing::Pairing, AffineRepr, CurveGroup};
use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

// This is weighted
#[derive(
    SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Debug, Clone, PartialEq, Eq,
)]
pub struct HkzgElgamalCommitWitness<F: PrimeField> {
    pub hkzg_randomness: univariate_hiding_kzg::CommitmentRandomness<F>,
    pub chunked_plaintexts: Vec<Vec<Vec<Scalar<F>>>>,
    pub elgamal_randomness: Vec<Vec<Scalar<F>>>,
    pub plaintexts: Vec<Vec<Scalar<F>>>,
}

type LiftedHkzgElgamalHomomorphism<'a, E> = LiftHomomorphism<
    hkzg_chunked_elgamal::WeightedHomomorphism<'a, E>,
    HkzgElgamalCommitWitness<<E as Pairing>::ScalarField>,
>;
type LiftedCommitHomomorphism<C> = LiftHomomorphism<
    scalar_mul::Homomorphism<C>,
    HkzgElgamalCommitWitness<<<C as CurveGroup>::Affine as AffineRepr>::ScalarField>,
>;

pub type Homomorphism<'a, E> = PairingTupleHomomorphism<
    E,
    LiftedHkzgElgamalHomomorphism<'a, E>,
    LiftedCommitHomomorphism<<E as Pairing>::G2>,
>;
#[allow(dead_code)]
pub type Proof<'a, E> = sigma_protocol::Proof<<E as Pairing>::ScalarField, Homomorphism<'a, E>>;

#[allow(non_snake_case)]
impl<'a, E: Pairing> Homomorphism<'a, E> {
    pub fn new(
        lagr_g1: &'a [E::G1Affine],
        xi_1: E::G1Affine,
        pp: &'a chunked_elgamal::PublicParameters<E>,
        eks: &'a [E::G1Affine],
        base: E::G2Affine,
    ) -> Self {
        // Set up the HKZG-EG homomorphism, and use a projection map to lift it to HkzgElgamalCommitWitness
        let lifted_hkzg_el_hom = LiftedHkzgElgamalHomomorphism::<E> {
            hom: hkzg_chunked_elgamal::WeightedHomomorphism::<E>::new(lagr_g1, xi_1, pp, eks),
            projection: |dom: &HkzgElgamalCommitWitness<E::ScalarField>| {
                HkzgWeightedElgamalWitness {
                    hkzg_randomness: dom.hkzg_randomness.clone(),
                    chunked_plaintexts: dom.chunked_plaintexts.clone(),
                    elgamal_randomness: dom.elgamal_randomness.clone(),
                }
            },
        };

        // Set up the lifted commit homomorphism
        let lifted_commit_hom = LiftedCommitHomomorphism::<E::G2> {
            hom: scalar_mul::Homomorphism { base },
            // The projection map simply ignores the `hkzg_randomness` component
            projection: |dom: &HkzgElgamalCommitWitness<E::ScalarField>| {
                scalar_mul::Witness {
                    values: dom
                        .plaintexts
                        .iter() // iterate over &Vec<Scalar<F>>
                        .flatten() // &Scalar<F>
                        .cloned() // convert &Scalar<F> -> Scalar<F>
                        .collect::<Vec<_>>(), // Vec<Scalar<F>>
                }
            },
        };

        // Combine the two lifted homomorphisms just constructed, into the required TupleHomomorphism
        Self {
            hom1: lifted_hkzg_el_hom,
            hom2: lifted_commit_hom,
            _pairing: std::marker::PhantomData,
        }
    }
}
