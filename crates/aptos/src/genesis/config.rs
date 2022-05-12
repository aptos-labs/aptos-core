// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::types::{CliError, CliTypedResult},
    genesis::git::from_yaml,
};
use aptos_config::config::HANDSHAKE_VERSION;
use aptos_crypto::{ed25519::Ed25519PublicKey, x25519};
use aptos_types::{
    chain_id::ChainId,
    network_address::{DnsName, NetworkAddress, Protocol},
    transaction::authenticator::AuthenticationKey,
};
use move_deps::move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::{
    convert::TryFrom,
    fs::File,
    io::Read,
    net::{Ipv4Addr, Ipv6Addr},
    path::PathBuf,
    str::FromStr,
};
use vm_genesis::Validator;

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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorConfiguration {
    /// Account address
    pub account_address: AccountAddress,
    /// Key used for signing in consensus
    pub consensus_key: Ed25519PublicKey,
    /// Key used for signing transactions with the account
    pub account_key: Ed25519PublicKey,
    /// Public key used for validator network identity (same as account address)
    pub validator_network_key: x25519::PublicKey,
    /// Host for validator which can be an IP or a DNS name
    pub validator_host: HostAndPort,
    /// Public key used for full node network identity (same as account address)
    pub full_node_network_key: x25519::PublicKey,
    /// Host for full node which can be an IP or a DNS name and is optional
    pub full_node_host: Option<HostAndPort>,
    /// Stake amount for consensus
    pub stake_amount: u64,
}

impl TryFrom<ValidatorConfiguration> for Validator {
    type Error = CliError;

    fn try_from(config: ValidatorConfiguration) -> Result<Self, CliError> {
        let auth_key = AuthenticationKey::ed25519(&config.account_key);
        let validator_addresses = vec![config
            .validator_host
            .as_network_address(config.validator_network_key)
            .unwrap()];
        let full_node_addresses = if let Some(full_node_host) = config.full_node_host {
            vec![full_node_host
                .as_network_address(config.full_node_network_key)
                .unwrap()]
        } else {
            vec![]
        };

        let derived_address = auth_key.derived_address();
        if config.account_address != derived_address {
            return Err(CliError::CommandArgumentError(format!(
                "AccountAddress {} does not match account key derived one {}",
                config.account_address, derived_address
            )));
        }
        Ok(Validator {
            address: derived_address,
            consensus_pubkey: config.consensus_key.to_bytes().to_vec(),
            operator_address: auth_key.derived_address(),
            network_address: bcs::to_bytes(&validator_addresses).unwrap(),
            full_node_network_address: bcs::to_bytes(&full_node_addresses).unwrap(),
            operator_auth_key: auth_key,
            auth_key,
            stake_amount: config.stake_amount,
        })
    }
}

/// Combined Host (DnsName or IP) and port
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostAndPort {
    pub host: DnsName,
    pub port: u16,
}

impl HostAndPort {
    pub fn as_network_address(&self, key: x25519::PublicKey) -> CliTypedResult<NetworkAddress> {
        let host = self.host.to_string();

        // Since DnsName supports IPs as well, let's properly fix what the type is
        let host_protocol = if let Ok(ip) = Ipv4Addr::from_str(&host) {
            Protocol::Ip4(ip)
        } else if let Ok(ip) = Ipv6Addr::from_str(&host) {
            Protocol::Ip6(ip)
        } else {
            Protocol::Dns(self.host.clone())
        };
        let port_protocol = Protocol::Tcp(self.port);
        let noise_protocol = Protocol::NoiseIK(key);
        let handshake_protocol = Protocol::Handshake(HANDSHAKE_VERSION);

        NetworkAddress::try_from(vec![
            host_protocol,
            port_protocol,
            noise_protocol,
            handshake_protocol,
        ])
        .map_err(|e| CliError::UnexpectedError(e.to_string()))
    }
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
