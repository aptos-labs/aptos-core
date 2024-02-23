// Copyright © Aptos Foundation

use aes_gcm::aead::rand_core::{CryptoRng as AeadCryptoRng, RngCore as AeadRngCore};
use rand_core::{CryptoRng, RngCore};

/// Implement this to define an asymmetric encryption scheme.
pub trait AsymmetricEncryption {
    fn scheme_name() -> String;

    /// Generate a key pair. Return `(private_key, public_key)`.
    fn key_gen<R: CryptoRng + RngCore>(rng: &mut R) -> (Vec<u8>, Vec<u8>);

    /// TODO: adjust the dependencies so they can share a RNG.
    fn enc<R1: CryptoRng + RngCore, R2: AeadCryptoRng + AeadRngCore>(
        rng: &mut R1,
        aead_rng: &mut R2,
        pk: &[u8],
        msg: &[u8],
    ) -> anyhow::Result<Vec<u8>>;

    fn dec(sk: &[u8], ciphertext: &[u8]) -> anyhow::Result<Vec<u8>>;
}

// /// An asymmetric encryption which:
// /// - uses AES-256-GCM to encrypt the original variable-length input, where the symmetric key is freshly sampled;
// /// - uses ElGamal over BLS12-381 G1 to encrypt the symmetric key.
// ///
// /// TODO: better name?
// pub mod scheme0;

/// An asymmetric encryption which:
/// - uses AES-256-GCM to encrypt the original variable-length input, where the symmetric key is freshly sampled;
/// - uses ElGamal over the group that supports ED25519 signatures to encrypt the symmetric key.
///
/// TODO: better name?
pub mod scheme1;
