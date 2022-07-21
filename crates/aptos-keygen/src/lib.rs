// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{
    bls12381,
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    x25519, CryptoMaterialError, PrivateKey, Uniform,
};
use aptos_types::{account_address::AccountAddress, transaction::authenticator::AuthenticationKey};
use rand::{
    rngs::{OsRng, StdRng},
    Rng, SeedableRng,
};

/// Ed25519 key generator.
#[derive(Debug)]
pub struct KeyGen(StdRng);

impl KeyGen {
    /// Constructs a key generator with a specific seed.
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self(StdRng::from_seed(seed))
    }

    /// Constructs a key generator with a random seed.
    /// The random seed itself is generated using the OS rng.
    pub fn from_os_rng() -> Self {
        let mut seed_rng = OsRng;
        let seed: [u8; 32] = seed_rng.gen();
        Self::from_seed(seed)
    }

    /// Generate an Ed25519 private key.
    pub fn generate_ed25519_private_key(&mut self) -> Ed25519PrivateKey {
        Ed25519PrivateKey::generate(&mut self.0)
    }

    /// Generate a bls12381 private key.
    pub fn generate_bls12381_private_key(&mut self) -> bls12381::PrivateKey {
        bls12381::PrivateKey::generate(&mut self.0)
    }

    /// Generate an Ed25519 key pair.
    pub fn generate_ed25519_keypair(&mut self) -> (Ed25519PrivateKey, Ed25519PublicKey) {
        let private_key = self.generate_ed25519_private_key();
        let public_key = private_key.public_key();
        (private_key, public_key)
    }

    /// Generate a x25519 private key.
    pub fn generate_x25519_private_key(
        &mut self,
    ) -> Result<x25519::PrivateKey, CryptoMaterialError> {
        let ed25519_private_key = self.generate_ed25519_private_key();
        x25519::PrivateKey::from_ed25519_private_bytes(&ed25519_private_key.to_bytes())
    }
    /// Generate a x25519 key pair.
    pub fn generate_x25519_keypair(
        &mut self,
    ) -> Result<(x25519::PrivateKey, x25519::PublicKey), CryptoMaterialError> {
        let private_key = self.generate_x25519_private_key()?;
        let public_key = private_key.public_key();
        Ok((private_key, public_key))
    }

    /// Same as `generate_keypair`, but returns a tuple of (private_key, auth_key, account_addr) instead.
    pub fn generate_credentials_for_account_creation(
        &mut self,
    ) -> (Ed25519PrivateKey, Vec<u8>, AccountAddress) {
        let (private_key, public_key) = self.generate_ed25519_keypair();
        let auth_key = AuthenticationKey::ed25519(&public_key).to_vec();
        let account_addr = AccountAddress::from_bytes(&auth_key).unwrap();
        (private_key, auth_key, account_addr)
    }
}
