// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This submodule implements the *public parameters* for this "chunked_elgamal_field" PVSS scheme.

use crate::{
    algebra::GroupGenerators,
    pvss::{
        chunked_elgamal_field::{
            chunked_elgamal,
            dealt_keys::{DecryptPrivKey, EncryptPubKey},
        },
        traits,
    },
    range_proofs::{dekart_univariate_v2, traits::BatchedRangeProof},
    utils::{self},
};
use aptos_crypto::{
    arkworks::{
        hashing,
        serialization::{ark_de, ark_se},
    },
    CryptoMaterialError, ValidCryptoMaterial,
};
use ark_ec::{pairing::Pairing, CurveGroup};
use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::rand::{thread_rng, CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::ops::Mul;

const DST: &[u8] = b"APTOS_CHUNKED_ELGAMAL_FIELD_PVSS_DST";

#[derive(
    CanonicalSerialize, Serialize, CanonicalDeserialize, Deserialize, Clone, Debug, PartialEq, Eq,
)]
#[allow(non_snake_case)]
pub struct PublicParameters<E: Pairing> {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub pp_elgamal: chunked_elgamal::PublicParameters<E>, // TODO: make this <E::G1> or <E::G1Affine> instead?
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub pk_range_proof: dekart_univariate_v2::ProverKey<E>,
    /// Base for the commitments to the polynomial evaluations (and for the dealt public key [shares])
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    G_2: E::G2Affine,
    #[serde(skip)]
    pub powers_of_radix: Vec<E::ScalarField>,
}

impl<E: Pairing> traits::HasEncryptionPublicParams for PublicParameters<E> {
    type EncryptionPublicParameters = chunked_elgamal::PublicParameters<E>;

    fn get_encryption_public_params(&self) -> &Self::EncryptionPublicParameters {
        &self.pp_elgamal
    }
}

impl<E: Pairing> traits::Convert<EncryptPubKey<E>, PublicParameters<E>> for DecryptPrivKey<E> {
    /// Given a decryption key $dk$, computes its associated encryption key $H^{dk}$
    fn to(&self, pp: &PublicParameters<E>) -> EncryptPubKey<E> {
        EncryptPubKey::<E> {
            ek: pp.pp_elgamal.pubkey_base().mul(self.dk).into_affine(),
        }
    }
}

#[allow(non_snake_case)]
impl<E: Pairing> TryFrom<&[u8]> for PublicParameters<E> {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        use ark_serialize::CanonicalDeserialize;

        let mut reader = bytes;

        // Deserialize pp_elgamal
        let pp_elgamal =
            chunked_elgamal::PublicParameters::<E>::deserialize_compressed(&mut reader)
                .map_err(|_| CryptoMaterialError::DeserializationError)?;

        // Deserialize pk_range_proof
        let pk_range_proof =
            dekart_univariate_v2::ProverKey::<E>::deserialize_compressed(&mut reader)
                .map_err(|_| CryptoMaterialError::DeserializationError)?;

        // Deserialize G_2
        let G_2 = E::G2Affine::deserialize_compressed(&mut reader)
            .map_err(|_| CryptoMaterialError::DeserializationError)?;

        // Recompute powers_of_radix
        let base = E::ScalarField::from(1u64 << 16); // TODO: change radix here
        let num_chunks = E::ScalarField::MODULUS_BIT_SIZE.div_ceil(16) as usize;
        let powers_of_radix = utils::powers(base, num_chunks + 1);

        Ok(PublicParameters {
            pp_elgamal,
            pk_range_proof,
            G_2,
            powers_of_radix,
        })
    }
}

#[allow(dead_code)]
impl<E: Pairing> PublicParameters<E> {
    /// Verifiably creates Aptos-specific public parameters.
    pub fn new<R: RngCore + CryptoRng>(
        max_num_shares: usize,
        radix_exponent: usize,
        rng: &mut R,
    ) -> Self {
        let max_num_chunks =
            max_num_shares * (E::ScalarField::MODULUS_BIT_SIZE as usize).div_ceil(radix_exponent);
        let max_num_chunks_padded = (max_num_chunks + 1).next_power_of_two() - 1;
        let base = E::ScalarField::from(1u64 << radix_exponent);
        let group_generators = GroupGenerators::sample(rng); // hmm at one of these should come from a powers of tau ceremony

        let pp = Self {
            pp_elgamal: chunked_elgamal::PublicParameters::default(),
            pk_range_proof: dekart_univariate_v2::Proof::setup(
                max_num_chunks_padded,
                radix_exponent,
                group_generators,
                rng,
            )
            .0,
            G_2: hashing::unsafe_hash_to_affine(b"G_2", DST), // TODO: fix DST
            powers_of_radix: utils::powers(base, max_num_chunks_padded + 1), // TODO: why the +1?
        };

        pp
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Serialize pp_elgamal
        self.pp_elgamal.serialize_compressed(&mut bytes).unwrap();

        // Serialize pk_range_proof
        self.pk_range_proof
            .serialize_compressed(&mut bytes)
            .unwrap();

        // Serialize G_2
        self.G_2.serialize_compressed(&mut bytes).unwrap();

        bytes
    }

    pub fn get_commitment_base(&self) -> &E::G2Affine {
        &self.G_2
    }
}

impl<E: Pairing> ValidCryptoMaterial for PublicParameters<E> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl<E: Pairing> Default for PublicParameters<E> {
    fn default() -> Self {
        let mut rng = thread_rng();
        Self::new(1, 16, &mut rng) // TODO: REFER TO CONSTANT HERE: build_constants::CHUNK_SIZE as usize
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use ark_bls12_381::Bls12_381;
//     use ark_std::rand::thread_rng;

//     #[test]
//     fn test_public_parameters_roundtrip() {
//         let mut rng = thread_rng();

//         // Create some public parameters
//         let pp_original = PublicParameters::<Bls12_381>::new(5, 16, &mut rng);

//         // Serialize to bytes
//         let serialized_bytes = pp_original.to_bytes();

//         // Deserialize back
//         let pp_deserialized = PublicParameters::<Bls12_381>::try_from(serialized_bytes.as_slice())
//             .expect("Deserialization failed");

//         // Check equality
//         assert_eq!(pp_original, pp_deserialized, "Roundtrip failed: deserialized parameters differ from original");
//     }

//     #[test]
//     fn test_public_parameters_serde_roundtrip() {
//         let mut rng = thread_rng();

//         // Create some public parameters
//         let pp_original = PublicParameters::<Bls12_381>::new(5, 16, &mut rng);

//         // Serialize with serde
//         let serialized = bcs::to_bytes(&pp_original).expect("Serde serialization failed");

//         // Deserialize with serde
//         let deserialized: PublicParameters<Bls12_381> =
//             bcs::from_bytes(&serialized).expect("Serde deserialization failed");

//         // Check equality
//         assert_eq!(pp_original, deserialized, "Serde roundtrip failed");
//         // THIS IS ALWAYS GOING TO FAIL. BETTER:
//         assert_eq!(pp_original.pp_elgamal, deserialized.pp_elgamal);
//         assert_eq!(pp_original.pk_range_proof, deserialized.pk_range_proof);
//         assert_eq!(pp_original.G_2, deserialized.G_2);
//     }
// }
