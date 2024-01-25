// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{config::SecureBackend, keys::ConfigKey};
use aptos_crypto::{bls12381, ed25519::Ed25519PrivateKey, x25519};
use aptos_types::{
    account_address::{AccountAddress, AccountAddress as PeerId},
    dkg::{DKGTrait, DefaultDKG},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

/// A single struct for reading / writing to a file for identity across configs
#[derive(Deserialize, Serialize)]
pub struct IdentityBlob {
    /// Optional account address. Used for validators and validator full nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_address: Option<AccountAddress>,
    /// Optional account key. Only used for validators
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_private_key: Option<Ed25519PrivateKey>,
    /// Optional consensus key. Only used for validators
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consensus_private_key: Option<bls12381::PrivateKey>,
    /// Network private key. Peer id is derived from this if account address is not present
    pub network_private_key: x25519::PrivateKey,
}

impl IdentityBlob {
    pub fn from_file(path: &Path) -> anyhow::Result<IdentityBlob> {
        Ok(serde_yaml::from_str(&fs::read_to_string(path)?)?)
    }

    pub fn to_file(&self, path: &Path) -> anyhow::Result<()> {
        let mut file = File::open(path)?;
        Ok(file.write_all(serde_yaml::to_string(self)?.as_bytes())?)
    }

    pub fn try_into_dkg_dealer_private_key(
        self,
    ) -> Option<<DefaultDKG as DKGTrait>::DealerPrivateKey> {
        self.consensus_private_key
    }

    pub fn try_into_dkg_new_validator_decrypt_key(
        self,
    ) -> Option<<DefaultDKG as DKGTrait>::NewValidatorDecryptKey> {
        self.consensus_private_key
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Identity {
    FromConfig(IdentityFromConfig),
    FromStorage(IdentityFromStorage),
    FromFile(IdentityFromFile),
    None,
}

impl Identity {
    pub fn from_config(key: x25519::PrivateKey, peer_id: PeerId) -> Self {
        let key = ConfigKey::new(key);
        Identity::FromConfig(IdentityFromConfig { key, peer_id })
    }

    pub fn from_storage(key_name: String, peer_id_name: String, backend: SecureBackend) -> Self {
        Identity::FromStorage(IdentityFromStorage {
            backend,
            key_name,
            peer_id_name,
        })
    }

    pub fn from_file(path: PathBuf) -> Self {
        Identity::FromFile(IdentityFromFile { path })
    }
}

/// The identity is stored within the config.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityFromConfig {
    #[serde(flatten)]
    pub key: ConfigKey<x25519::PrivateKey>,
    pub peer_id: PeerId,
}

/// This represents an identity in a secure-storage as defined in NodeConfig::secure.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityFromStorage {
    pub backend: SecureBackend,
    pub key_name: String,
    pub peer_id_name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityFromFile {
    pub path: PathBuf,
}
