// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{bls12381, ed25519::Ed25519PrivateKey, x25519};
use aptos_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::{fs, fs::File, io::Write, path::Path};

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
}
