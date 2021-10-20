// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    PrivateKey, Uniform,
};
use diem_types::{account_address::AccountAddress, transaction::authenticator::AuthenticationKey};
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

    /// Generate an Ed25519 key pair.
    pub fn generate_keypair(&mut self) -> (Ed25519PrivateKey, Ed25519PublicKey) {
        let private_key = Ed25519PrivateKey::generate(&mut self.0);
        let public_key = private_key.public_key();
        (private_key, public_key)
    }

    /// Same as `generate_keypair`, but returns a tuple of (private_key, auth_key_prefix, account_addr) instead.
    pub fn generate_credentials_for_account_creation(
        &mut self,
    ) -> (Ed25519PrivateKey, Vec<u8>, AccountAddress) {
        let (private_key, public_key) = self.generate_keypair();
        let auth_key = AuthenticationKey::ed25519(&public_key).to_vec();
        const AUTH_KEY_PREFIX_LENGTH: usize = AuthenticationKey::LENGTH - AccountAddress::LENGTH;
        let auth_key_prefix = auth_key[..AUTH_KEY_PREFIX_LENGTH].to_vec();
        let account_addr = AccountAddress::from_bytes(&auth_key[AUTH_KEY_PREFIX_LENGTH..]).unwrap();
        (private_key, auth_key_prefix, account_addr)
    }
}
