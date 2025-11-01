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

impl<E: Pairing> TryFrom<&[u8]> for PublicParameters<E> {
    type Error = CryptoMaterialError;

    fn try_from(_bytes: &[u8]) -> Result<PublicParameters<E>, Self::Error> {
        todo!("Deserialization of PublicParameters from bytes not yet implemented");
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
        let num_chunks = max_num_shares * 255usize.div_ceil(radix_exponent);
        let num_chunks_padded = (num_chunks + 1).next_power_of_two() - 1;
        let base = E::ScalarField::from(1u64 << radix_exponent);
        let group_generators = GroupGenerators::sample(rng); // hmm at one of these should come from a powers of tau ceremony

        let pp = Self {
            pp_elgamal: chunked_elgamal::PublicParameters::default(),
            pk_range_proof: dekart_univariate_v2::Proof::setup(
                radix_exponent,
                num_chunks_padded,
                group_generators,
                rng,
            )
            .0,
            G_2: hashing::unsafe_hash_to_affine(b"G_2", DST), // TODO: fix DST
            powers_of_radix: utils::powers(base, num_chunks_padded + 1), // TODO: why the +1?
        };

        pp
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let bytes = Vec::new();
        // bytes.extend_from_slice(&self.pp_elgamal.to_bytes()); // has constant size
        // bytes.extend_from_slice(&self.g_2.to_compressed()); // has constant size
        // bytes.extend_from_slice(&self.pp_range_proof.to_bytes());
        // The powers of B need not be serialized, they can just be recomputed during deserialization

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
