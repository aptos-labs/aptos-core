// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This file implements a KeyPair data structure.
//!
//! The point of a KeyPair is to deserialize a private key into a structure
//! that will only allow the private key to be moved out once
//! (hence providing good key hygiene)
//! while allowing access to the public key part forever.
//!
//! The public key part is dynamically derived during deserialization,
//! while ignored during serialization.
//!

use velor_crypto::{
    CryptoMaterialError, PrivateKey, ValidCryptoMaterial, ValidCryptoMaterialStringExt,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// ConfigKey places a clonable wrapper around PrivateKeys for config purposes only. The only time
/// configs have keys is either for testing or for low security requirements. We recommend that
/// keys be stored in key managers. If we make keys unclonable, then the configs must be mutable
/// and that becomes a requirement strictly as a result of supporting test environments, which is
/// undesirable. Hence this internal wrapper allows for keys to be clonable but only from configs.
#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigKey<T: PrivateKey + Serialize> {
    #[serde(bound(deserialize = "T: Deserialize<'de>"))]
    key: T,
}

impl<T: DeserializeOwned + PrivateKey + ValidCryptoMaterial + Serialize> ConfigKey<T> {
    pub fn new(key: T) -> Self {
        Self { key }
    }

    pub fn private_key(&self) -> T {
        self.clone().key
    }

    pub fn public_key(&self) -> T::PublicKeyMaterial {
        velor_crypto::PrivateKey::public_key(&self.key)
    }

    pub fn from_encoded_string(str: &str) -> Result<Self, CryptoMaterialError> {
        Ok(Self::new(T::from_encoded_string(str)?))
    }
}

impl<T: DeserializeOwned + PrivateKey + Serialize> Clone for ConfigKey<T> {
    fn clone(&self) -> Self {
        bcs::from_bytes(&bcs::to_bytes(self).unwrap()).unwrap()
    }
}

#[cfg(test)]
impl<T: PrivateKey + Serialize + velor_crypto::Uniform> Default for ConfigKey<T> {
    fn default() -> Self {
        Self {
            key: velor_crypto::Uniform::generate_for_testing(),
        }
    }
}

impl<T: PrivateKey + Serialize> PartialEq for ConfigKey<T> {
    fn eq(&self, other: &Self) -> bool {
        bcs::to_bytes(&self).unwrap() == bcs::to_bytes(&other).unwrap()
    }
}
