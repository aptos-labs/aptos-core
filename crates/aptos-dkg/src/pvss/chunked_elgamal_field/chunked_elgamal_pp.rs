// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use ark_ec::pairing::Pairing;
use ark_serialize::CanonicalSerialize;
use ark_serialize::CanonicalDeserialize;
use aptos_crypto_derive::SilentDisplay;
use aptos_crypto_derive::SilentDebug;
use aptos_crypto::Uniform;
use ark_ff::UniformRand;
use aptos_crypto::arkworks::hashing;

pub const DST_PVSS_PUBLIC_PARAMS: &[u8; 30] = b"APTOS_CHUNKED_ELGAMAL_PVSS_DST";

#[derive(CanonicalSerialize, CanonicalDeserialize, PartialEq, Clone, Eq, Debug)]
#[allow(non_snake_case)]
pub struct PublicParameters<E: Pairing> {
    /// A group element $G that is raised to the encrypted message
    pub G: E::G1Affine,
    /// A group element $H$ that is used to exponentiate
    /// both the (1) ciphertext randomness and the (2) the DSK when computing its EK.
    pub H: E::G1Affine,
}

/// The *encryption (public)* key used to encrypt shares of the dealt secret for each PVSS player.
#[derive(Clone, PartialEq, Eq)]
pub struct EncryptPubKey<E: Pairing> {
    /// A group element $H^{dk^{-1}} \in G_1$.
    pub(crate) ek: E::G1Affine,
}

impl<E: Pairing> Uniform for DecryptPrivKey<E> {
    fn generate<R>(_rng: &mut R) -> Self
    where
        R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
    {
        DecryptPrivKey::<E> {
            dk: E::ScalarField::rand(&mut ark_std::rand::thread_rng()), // Workaround because the `rand` versions differ
        }
    }
}

/// The *decryption (secret) key* used by each PVSS player to decrypt their share of the dealt secret.
#[derive(SilentDisplay, SilentDebug)]
pub struct DecryptPrivKey<E: Pairing> {
    /// A scalar $dk \in F$.
    pub(crate) dk: E::ScalarField,
}

#[allow(non_snake_case)]
impl<E: Pairing> PublicParameters<E> {
    pub fn new(g: E::G1Affine, h: E::G1Affine) -> Self {
        Self { G: g, H: h }
    }

    // pub fn to_bytes(&self) -> [u8; 2 * $GT_PROJ_NUM_BYTES] {
    //     let mut bytes = [0u8; 2 * $GT_PROJ_NUM_BYTES];

    //     // Copy bytes from g.to_compressed() into the first half of the bytes array.
    //     bytes[..$GT_PROJ_NUM_BYTES].copy_from_slice(&self.g.to_compressed());

    //     // Copy bytes from h.to_compressed() into the second half of the bytes array.
    //     bytes[$GT_PROJ_NUM_BYTES..].copy_from_slice(&self.h.to_compressed());

    //     bytes
    // }


    pub fn message_base(&self) -> &E::G1Affine {
        &self.G
    }

    pub fn pubkey_base(&self) -> &E::G1Affine {
        &self.H
    }

    pub fn default() -> Self {
        let G = hashing::unsafe_hash_to_affine(b"G", DST_PVSS_PUBLIC_PARAMS);
        let H = hashing::unsafe_hash_to_affine(b"H", DST_PVSS_PUBLIC_PARAMS);
        debug_assert_ne!(G, H);
        Self { G, H}
    }
}