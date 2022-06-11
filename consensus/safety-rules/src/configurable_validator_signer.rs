// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{Error, PersistentSafetyStorage};
use aptos_crypto::{bls12381, hash::CryptoHash};
use aptos_types::{account_address::AccountAddress, validator_signer::ValidatorSigner};
use serde::Serialize;

/// A ConfigurableValidatorSigner is a ValidatorSigner wrapper that offers either
/// a ValidatorSigner instance or a ValidatorHandle instance, depending on the
/// configuration chosen. This abstracts away the complexities of handling either
/// instance, while offering the same API as a ValidatorSigner.
pub enum ConfigurableValidatorSigner {
    Signer(ValidatorSigner),
    // Handle(ValidatorHandle),
}

impl ConfigurableValidatorSigner {
    /// Returns a new ValidatorSigner instance
    pub fn new_signer(author: AccountAddress, consensus_key: bls12381::PrivateKey) -> Self {
        let signer = ValidatorSigner::new(author, consensus_key);
        ConfigurableValidatorSigner::Signer(signer)
    }

    /// Returns a new ValidatorHandle instance
    // pub fn new_handle(author: AccountAddress, key_version: bls12381::PublicKey) -> Self {
    //     let handle = ValidatorHandle::new(author, key_version);
    //     ConfigurableValidatorSigner::Handle(handle)
    // }

    /// Returns the author associated with the signer configuration.
    pub fn author(&self) -> AccountAddress {
        match self {
            ConfigurableValidatorSigner::Signer(signer) => signer.author(),
            // ConfigurableValidatorSigner::Handle(handle) => handle.author(),
        }
    }

    /// Returns the public key associated with the signer configuration.
    pub fn public_key(&self) -> bls12381::PublicKey {
        match self {
            ConfigurableValidatorSigner::Signer(signer) => signer.public_key(),
            // ConfigurableValidatorSigner::Handle(handle) => handle.key_version(),
        }
    }

    /// Signs a given message using the signer configuration.
    pub fn sign<T: Serialize + CryptoHash>(
        &self,
        message: &T,
        _storage: &PersistentSafetyStorage,
    ) -> Result<bls12381::Signature, Error> {
        match self {
            ConfigurableValidatorSigner::Signer(signer) => Ok(signer.sign(message)),
            // ConfigurableValidatorSigner::Handle(handle) => handle.sign(message, storage),
        }
    }
}
