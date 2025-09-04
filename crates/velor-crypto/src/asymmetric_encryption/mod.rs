// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides asymmetric encryption traits and instances.

use aes_gcm::aead::rand_core::{CryptoRng as AeadCryptoRng, RngCore as AeadRngCore};
use rand_core::{CryptoRng, RngCore};

/// Implement this to define an asymmetric encryption scheme.
pub trait AsymmetricEncryption {
    /// A.k.a the decrypt key.
    type PrivateKey;

    /// A.k.a the encrypt key.
    type PublicKey;

    /// The name of the scheme.
    fn scheme_name() -> String;

    /// Generate a key pair. Return `(private_key, public_key)`.
    fn key_gen<R: CryptoRng + RngCore>(rng: &mut R) -> (Self::PrivateKey, Self::PublicKey);

    /// The encryption algorithm.
    /// TODO: adjust the dependencies so they can share a RNG.
    fn enc<R1: CryptoRng + RngCore, R2: AeadCryptoRng + AeadRngCore>(
        rng: &mut R1,
        aead_rng: &mut R2,
        pk: &Self::PublicKey,
        msg: &[u8],
    ) -> anyhow::Result<Vec<u8>>;

    /// The decryption algorithm.
    fn dec(sk: &Self::PrivateKey, ciphertext: &[u8]) -> anyhow::Result<Vec<u8>>;
}

/// An asymmetric encryption which:
/// - uses AES-256-GCM to encrypt the original variable-length input, where the symmetric key is freshly sampled;
/// - uses ElGamal over the group that supports ED25519 signatures to encrypt the symmetric key.
pub mod elgamal_curve25519_aes256_gcm;
