// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::types::{CliError, CliTypedResult},
    genesis::git::from_yaml,
};
use aptos_crypto::{ed25519::Ed25519PublicKey, x25519};
use aptos_types::{chain_id::ChainId, network_address::DnsName};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::PathBuf, str::FromStr};

/// Template for setting up Github for Genesis
///
#[derive(Debug, Deserialize, Serialize)]
pub struct Layout {
    /// Root key for the blockchain
    /// TODO: In the future, we won't need a root key
    pub root_key: Ed25519PublicKey,
    /// List of usernames or identifiers
    pub users: Vec<String>,
    /// ChainId for the target network
    pub chain_id: ChainId,
    /// Modules folder
    pub modules_folder: String,
}

impl Layout {
    /// Read the layout from a YAML file on disk
    pub fn from_disk(path: &PathBuf) -> CliTypedResult<Self> {
        let mut file =
            File::open(&path).map_err(|e| CliError::IO(path.display().to_string(), e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| CliError::IO(path.display().to_string(), e))?;
        from_yaml(&contents)
    }
}

/// A set of configuration needed to add a Validator to genesis
///
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidatorConfiguration {
    /// Key used for signing in consensus
    pub consensus_key: Ed25519PublicKey,
    /// Key used for signing transactions with the account
    pub account_key: Ed25519PublicKey,
    /// Public key used for network identity (same as account address)
    pub network_key: x25519::PublicKey,
    /// Host for validator which can be an IP or a DNS name
    pub validator_host: HostAndPort,
    /// Host for full node which can be an IP or a DNS name and is optional
    pub full_node_host: Option<HostAndPort>,
}

/// Combined Host (DnsName or IP) and port
#[derive(Debug, Serialize, Deserialize)]
pub struct HostAndPort {
    pub host: DnsName,
    pub port: u16,
}

impl FromStr for HostAndPort {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split(':').collect();
        if parts.len() != 2 {
            Err(CliError::CommandArgumentError(
                "Invalid host and port, must be of the form 'host:port` e.g. '127.0.0.1:6180'"
                    .to_string(),
            ))
        } else {
            let host = DnsName::from_str(*parts.get(0).unwrap())
                .map_err(|e| CliError::CommandArgumentError(e.to_string()))?;
            let port = u16::from_str(parts.get(1).unwrap())
                .map_err(|e| CliError::CommandArgumentError(e.to_string()))?;
            Ok(HostAndPort { host, port })
        }
    }
}
