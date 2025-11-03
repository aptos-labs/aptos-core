// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This submodule implements the *public parameters* for this "chunked_elgamal_field" PVSS scheme.

use crate::{
    algebra::GroupGenerators,
    pvss::{
        chunked_elgamal_field::{chunked_elgamal, input_secret::InputSecret, keys},
        traits,
    },
    range_proofs::{dekart_univariate_v2, traits::BatchedRangeProof},
};
use aptos_crypto::{
    arkworks::{
        self, hashing,
        serialization::{ark_de, ark_se},
    },
    CryptoMaterialError, ValidCryptoMaterial,
};
use ark_ec::{pairing::Pairing, CurveGroup};
use ark_ff::PrimeField;
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, Read, SerializationError, Valid, Validate,
};
use ark_std::rand::{thread_rng, CryptoRng, RngCore};
use serde::{Deserialize, Deserializer, Serialize};
use std::ops::Mul;

const DST: &[u8] = b"APTOS_CHUNKED_ELGAMAL_FIELD_PVSS_DST";

pub const CHUNK_SIZE: u8 = 16; // Will probably move this elsewhere in the next PR. Maybe it becomes a run-time variable...

fn compute_powers_of_radix<E: Pairing>() -> Vec<E::ScalarField> {
    let num_chunks_per_share =
        E::ScalarField::MODULUS_BIT_SIZE.div_ceil(CHUNK_SIZE as u32) as usize;
    arkworks::powers_of_two(num_chunks_per_share)
}

#[derive(CanonicalSerialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct PublicParameters<E: Pairing> {
    #[serde(serialize_with = "ark_se")]
    pub pp_elgamal: chunked_elgamal::PublicParameters<E>, // TODO: make this <E::G1> or <E::G1Affine> instead of <E>?

    #[serde(serialize_with = "ark_se")]
    pub pk_range_proof: dekart_univariate_v2::ProverKey<E>,

    /// Base for the commitments to the polynomial evaluations (and for the dealt public key [shares])
    #[serde(serialize_with = "ark_se")]
    G_2: E::G2Affine,

    #[serde(skip)]
    pub powers_of_radix: Vec<E::ScalarField>,
}

impl<E: Pairing> PublicParameters<E> {
    pub fn get_commitment_base(&self) -> E::G2Affine {
        self.G_2
    }
}

#[allow(non_snake_case)]
impl<'de, E: Pairing> Deserialize<'de> for PublicParameters<E> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize the serializable fields directly
        #[derive(Deserialize)]
        struct SerializedFields<E: Pairing> {
            #[serde(deserialize_with = "ark_de")]
            pp_elgamal: chunked_elgamal::PublicParameters<E>,
            #[serde(deserialize_with = "ark_de")]
            pk_range_proof: dekart_univariate_v2::ProverKey<E>,
            #[serde(deserialize_with = "ark_de")]
            G_2: E::G2Affine,
        }

        let serialized = SerializedFields::<E>::deserialize(deserializer)?;

        Ok(Self {
            pp_elgamal: serialized.pp_elgamal,
            pk_range_proof: serialized.pk_range_proof,
            G_2: serialized.G_2,
            powers_of_radix: compute_powers_of_radix::<E>(),
        })
    }
}

#[allow(non_snake_case)]
impl<E: Pairing> CanonicalDeserialize for PublicParameters<E> {
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        compress: Compress,
        validate: Validate,
    ) -> Result<Self, SerializationError> {
        let pp_elgamal = chunked_elgamal::PublicParameters::<E>::deserialize_with_mode(
            &mut reader,
            compress,
            validate,
        )?;
        let pk_range_proof = dekart_univariate_v2::ProverKey::<E>::deserialize_with_mode(
            &mut reader,
            compress,
            validate,
        )?;
        let G_2 = E::G2Affine::deserialize_with_mode(&mut reader, compress, validate)?;

        Ok(Self {
            pp_elgamal,
            pk_range_proof,
            G_2,
            powers_of_radix: compute_powers_of_radix::<E>(),
        })
    }
}

impl<E: Pairing> Valid for PublicParameters<E> {
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

impl<E: Pairing> traits::HasEncryptionPublicParams for PublicParameters<E> {
    type EncryptionPublicParameters = chunked_elgamal::PublicParameters<E>;

    fn get_encryption_public_params(&self) -> &Self::EncryptionPublicParameters {
        &self.pp_elgamal
    }
}

impl<E: Pairing> traits::Convert<keys::DealtPubKey<E>, PublicParameters<E>>
    for InputSecret<E::ScalarField>
{
    /// Computes the public key associated with the given input secret.
    /// NOTE: In the SCRAPE PVSS, a `DealtPublicKey` cannot be computed from a `DealtSecretKey` directly.
    fn to(&self, pp: &PublicParameters<E>) -> keys::DealtPubKey<E> {
        keys::DealtPubKey::new(
            pp.get_commitment_base()
                .mul(self.get_secret_a())
                .into_affine(),
        )
    }
}

#[allow(non_snake_case)]
impl<E: Pairing> TryFrom<&[u8]> for PublicParameters<E> {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<PublicParameters<E>>(bytes)
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

#[allow(dead_code)]
impl<E: Pairing> PublicParameters<E> {
    /// Verifiably creates Aptos-specific public parameters.
    pub fn new<R: RngCore + CryptoRng>(
        max_num_shares: usize,
        _radix_exponent: usize,
        rng: &mut R,
    ) -> Self {
        let num_chunks_per_share =
            E::ScalarField::MODULUS_BIT_SIZE.div_ceil(CHUNK_SIZE as u32) as usize;
        let max_num_chunks_padded =
            ((max_num_shares * num_chunks_per_share) + 1).next_power_of_two() - 1;

        let group_generators = GroupGenerators::sample(rng); // TODO: At least one of these should come from a powers of tau ceremony?
        let pp = Self {
            pp_elgamal: chunked_elgamal::PublicParameters::default(),
            pk_range_proof: dekart_univariate_v2::Proof::setup(
                max_num_chunks_padded,
                CHUNK_SIZE as usize,
                group_generators,
                rng,
            )
            .0,
            G_2: hashing::unsafe_hash_to_affine(b"G_2", DST),
            powers_of_radix: arkworks::powers_of_two(num_chunks_per_share),
        };

        pp
    }
}

impl<E: Pairing> ValidCryptoMaterial for PublicParameters<E> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("unexpected error during PVSS transcript serialization")
    }
}

impl<E: Pairing> Default for PublicParameters<E> {
    fn default() -> Self {
        let mut rng = thread_rng();
        Self::new(1, CHUNK_SIZE as usize, &mut rng)
    }
}
