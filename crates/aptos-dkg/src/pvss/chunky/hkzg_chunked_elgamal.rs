// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    pcs::univariate_hiding_kzg,
    pvss::chunky::chunked_elgamal,
    sigma_protocol::{
        self,
        homomorphism::{
            tuple::{TupleCodomainShape, TupleHomomorphism},
            LiftHomomorphism, TrivialShape,
        },
        traits::FirstProofItem,
    },
    Scalar,
};
use aptos_crypto::{
    arkworks::random::{
        sample_field_element, sample_field_elements, unsafe_random_point, unsafe_random_points,
        UniformRand,
    },
    weighted_config::WeightedConfigArkworks,
    SecretSharingConfig,
};
use aptos_crypto_derive::SigmaProtocolWitness;
use ark_ec::{pairing::Pairing, AdditiveGroup};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

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
    pub hkzg_randomness: univariate_hiding_kzg::CommitmentRandomness<E>,
    pub chunked_plaintexts: Vec<Vec<Scalar<E>>>, // For each plaintext z_i, a chunk z_{i,j}
    pub elgamal_randomness: Vec<Scalar<E>>,      // For each chunk, a blinding factor
}

#[derive(
    SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Debug, Clone, PartialEq, Eq,
)]
pub struct HkzgWeightedElgamalWitness<E: Pairing> {
    pub hkzg_randomness: univariate_hiding_kzg::CommitmentRandomness<E>,
    pub chunked_plaintexts: Vec<Vec<Vec<Scalar<E>>>>, // For each player, plaintexts z_i, which are chunked z_{i,j}
    pub elgamal_randomness: Vec<Vec<Scalar<E>>>, // For at most max_weight, for each chunk, a blinding factor
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

type LiftedHkzgWeighted<'a, E> = LiftHomomorphism<
    univariate_hiding_kzg::CommitmentHomomorphism<'a, E>,
    HkzgWeightedElgamalWitness<E>,
>;
type LiftedWeightedChunkedElgamal<'a, E> =
    LiftHomomorphism<chunked_elgamal::WeightedHomomorphism<'a, E>, HkzgWeightedElgamalWitness<E>>;

//                                 ┌───────────────────────────────┐
//                                 │     HkzgElgamalWitness<E>     │
//                                 │-------------------------------│
//                                 │ hkzg_randomness               │
//                                 │ chunked_plaintexts            │
//                                 │ elgamal_randomness            │
//                                 └───────────────┬───────────────┘
//                                                 │
//              ┌────────────────────────────────┬─╫─┬──────────────────────────┐
//              │                                ║ ╫ ║                          │
// projection_1 │         lifted HKZG hom ╔══════╝ ╫ ╚══════╗ lifted Chunked    │ projection_2
//              │                         ║        ╫        ║ ElGamal hom       │
//              ▼                         ║        ╫        ║                   ▼
//  ┌───────────────────────────────────┐ ║        ╫        ║  ┌──────────────────────────────┐
//  │ univariate_hiding_kzg::Witness<E> │ ║        ╫        ║  │ chunked_elgamal:: Witness<E> │
//  │-----------------------------------│ ║        ╫        ║  │------------------------------│
//  │ hkzg_randomness                   │ ║        ╫        ║  │ chunked_plaintexts           │
//  │ flattened_chunked_plaintexts      │ ║        ╫        ║  │ elgamal_randomness           │
//  └──────────────┬────────────────────┘ ║        ╫        ║  └──────────────┬───────────────┘
//                 │ ╔════════════════════╝        ╫        ╚═══════════════╗ │
//       HKZG hom  │ ║                             ╫                        ║ │ Chunked ElGamal hom
//                 │ ║                             ╫ TupleHomomorphism      ║ │
//                 ▼ ▼                             ╫                        ▼ ▼
//   ┌──────────────────────────┐                  ╫         ┌──────────────────────────┐
//   │ HKZG output (commitment) │                  ╫         │ Chunked ElGamal output   │
//   └──────────────┬───────────┘                  ╫         └──────────────┬───────────┘
//                  │                              ╫                        │
//                  └─────────────────────────────►╫◄───────────────────────┘
//                                                 ╫
//                                                 ▼
//                                  ┌──────────────────────────────────┐
//                                  │   TupleHomomorphism output       │
//                                  │   (pair of HKZG image and        │
//                                  │    Chunked ElGamal image)        │
//                                  └──────────────────────────────────┘
//
//
// In other words, the tuple homomorphism is roughly given as follows:
//
// ( rho, z_{i,j} , r_j ) │----> ( HKZG(rho, (0, z_{i,j}) ) , chunked_elgamal( z_{i,j} , r_j )
//                             = (
//			                         \rho [\xi]_1 + \sum_i,j z_i,j [\ell_{i * B + j + 1}(\tau)]_1,
//                                  ( z_i,j G_1 + r_j ek_i )_i,j,
//                                  ( r_j H_1 )_j,
//			                   )
// where B denotes the number of chunks.
//
// TODO: note here that we had to put a zero before z_{i,j}, because that's what DeKARTv2 is doing. So maybe
// it would make more sense to say this is a tuple homomorphism consisting of (lifts of) the
// DeKARTv2::commitment_homomorphism together with the chunked_elgamal::homomorphism.
pub type Homomorphism<'a, E> = TupleHomomorphism<LiftedHkzg<'a, E>, LiftedChunkedElgamal<'a, E>>;
pub type WeightedHomomorphism<'a, E> =
    TupleHomomorphism<LiftedHkzgWeighted<'a, E>, LiftedWeightedChunkedElgamal<'a, E>>;

pub type Proof<'a, E> = sigma_protocol::Proof<E, Homomorphism<'a, E>>;
pub type WeightedProof<'a, E> = sigma_protocol::Proof<E, WeightedHomomorphism<'a, E>>;

impl<'a, E: Pairing> Proof<'a, E> {
    /// Generates a random looking proof (but not a valid one).
    /// Useful for testing and benchmarking.
    pub fn generate<R: rand::Rng + rand::CryptoRng>(
        n: usize,
        number_of_chunks: usize,
        rng: &mut R,
    ) -> Self {
        // or should number_of_chunks be a const?
        Self {
            first_proof_item: FirstProofItem::Commitment(TupleCodomainShape(
                TrivialShape(unsafe_random_point(rng)), // because TrivialShape is the codomain of univariate_hiding_kzg::CommitmentHomomorphism. TODO: develop generate() methods there? Maybe make it part of sigma_protocol::Trait ?
                chunked_elgamal::CodomainShape {
                    chunks: vec![vec![unsafe_random_point(rng); number_of_chunks]; n],
                    randomness: vec![unsafe_random_point(rng); number_of_chunks],
                },
            )),
            z: HkzgElgamalWitness {
                hkzg_randomness: univariate_hiding_kzg::CommitmentRandomness::<E>::rand(rng),
                chunked_plaintexts: vec![
                    vec![Scalar(sample_field_element(rng)); number_of_chunks];
                    n
                ],
                elgamal_randomness: vec![Scalar(sample_field_element(rng)); number_of_chunks],
            },
        }
    }
}

impl<'a, E: Pairing> WeightedProof<'a, E> {
    /// Generates a random looking proof (but not a valid one).
    /// Useful for testing and benchmarking.
    pub fn generate<R: rand::Rng + rand::CryptoRng>(
        sc: &WeightedConfigArkworks<E::ScalarField>,
        number_of_chunks: usize,
        rng: &mut R,
    ) -> Self {
        // or should number_of_chunks be a const?
        Self {
            first_proof_item: FirstProofItem::Commitment(TupleCodomainShape(
                TrivialShape(unsafe_random_point(rng)), // because TrivialShape is the codomain of univariate_hiding_kzg::CommitmentHomomorphism. TODO: develop generate() methods there? Maybe make it part of sigma_protocol::Trait ?
                chunked_elgamal::WeightedCodomainShape {
                    chunks: (0..sc.get_total_num_players())
                        .map(|i| {
                            let w = sc.get_player_weight(&sc.get_player(i)); // TODO: combine these functions...
                            (0..w)
                                .map(|_| unsafe_random_points(number_of_chunks, rng))
                                .collect()
                        })
                        .collect(),
                    randomness: vec![
                        unsafe_random_points(number_of_chunks, rng);
                        sc.get_max_weight()
                    ],
                },
            )),
            z: HkzgWeightedElgamalWitness {
                hkzg_randomness: univariate_hiding_kzg::CommitmentRandomness::<E>::rand(rng),
                chunked_plaintexts: (0..sc.get_total_num_players())
                    .map(|i| {
                        let w = sc.get_player_weight(&sc.get_player(i)); // TODO: combine these functions...
                        (0..w)
                            .map(|_| {
                                Scalar::vec_from_inner(sample_field_elements(number_of_chunks, rng))
                            })
                            .collect()
                    })
                    .collect(),
                elgamal_randomness: vec![
                    vec![Scalar(sample_field_element(rng)); number_of_chunks];
                    sc.get_max_weight()
                ],
            },
        }
    }
}

#[allow(non_snake_case)]
impl<'a, E: Pairing> Homomorphism<'a, E> {
    pub fn new(
        lagr_g1: &'a [E::G1Affine],
        xi_1: E::G1Affine,
        pp: &'a chunked_elgamal::PublicParameters<E>,
        eks: &'a [E::G1Affine],
    ) -> Self {
        // Set up the HKZG homomorphism, and use a projection map to lift it to HkzgElgamalWitness
        let lifted_hkzg = LiftedHkzg::<E> {
            hom: univariate_hiding_kzg::CommitmentHomomorphism { lagr_g1, xi_1 },
            // The projection map ignores the `elgamal_randomness` component, and flattens the vector of chunked plaintexts after adding a zero
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
                    hiding_randomness: hkzg_randomness.clone(),
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

        // Combine the two lifted homomorphisms just constructed, into the required TupleHomomorphism
        Self {
            hom1: lifted_hkzg,
            hom2: lifted_chunked_elgamal,
        }
    }
}

#[allow(non_snake_case)]
impl<'a, E: Pairing> WeightedHomomorphism<'a, E> {
    pub fn new(
        lagr_g1: &'a [E::G1Affine],
        xi_1: E::G1Affine,
        pp: &'a chunked_elgamal::PublicParameters<E>,
        eks: &'a [E::G1Affine],
    ) -> Self {
        // Set up the HKZG homomorphism, and use a projection map to lift it to HkzgElgamalWitness
        let lifted_hkzg = LiftedHkzgWeighted::<E> {
            hom: univariate_hiding_kzg::CommitmentHomomorphism { lagr_g1, xi_1 },
            // The projection map ignores the `elgamal_randomness` component, and flattens the vector of chunked plaintexts after adding a zero
            projection: |dom: &HkzgWeightedElgamalWitness<E>| {
                let HkzgWeightedElgamalWitness {
                    hkzg_randomness,
                    chunked_plaintexts,
                    ..
                } = dom;
                let flattened_chunked_plaintexts: Vec<Scalar<E>> =
                    std::iter::once(Scalar(E::ScalarField::ZERO))
                        .chain(chunked_plaintexts.iter().flatten().flatten().cloned())
                        .collect();
                univariate_hiding_kzg::Witness::<E> {
                    hiding_randomness: hkzg_randomness.clone(),
                    values: flattened_chunked_plaintexts,
                }
            },
        };
        // Set up the chunked_elgamal homomorphism, and use a projection map to lift it to HkzgElgamalWitness
        let lifted_chunked_elgamal = LiftedWeightedChunkedElgamal::<E> {
            hom: chunked_elgamal::WeightedHomomorphism { pp, eks },
            // The projection map simply ignores the `hkzg_randomness` component
            projection: |dom: &HkzgWeightedElgamalWitness<E>| {
                let HkzgWeightedElgamalWitness {
                    chunked_plaintexts,
                    elgamal_randomness,
                    ..
                } = dom;
                chunked_elgamal::WeightedWitness {
                    plaintext_chunks: chunked_plaintexts.clone(),
                    plaintext_randomness: elgamal_randomness.clone(),
                }
            },
        };

        // Combine the two lifted homomorphisms just constructed, into the required TupleHomomorphism
        Self {
            hom1: lifted_hkzg,
            hom2: lifted_chunked_elgamal,
        }
    }
}
