// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::HANDSHAKE_VERSION;
use aptos_crypto::{bls12381, ed25519::Ed25519PublicKey, x25519};
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    network_address::{DnsName, NetworkAddress, Protocol},
    transaction::authenticator::AuthenticationKey,
};
use serde::{Deserialize, Serialize};
use std::{
    convert::TryFrom,
    fs::File,
    io::Read,
    net::{Ipv4Addr, Ipv6Addr, ToSocketAddrs},
    path::Path,
    str::FromStr,
};
use vm_genesis::{Validator, ValidatorWithCommissionRate};

/// Template for setting up Github for Genesis
///
#[derive(Debug, Deserialize, Serialize)]
pub struct Layout {
    /// Root key for the blockchain
    /// TODO: In the future, we won't need a root key
    pub root_key: Option<Ed25519PublicKey>,
    /// List of usernames or identifiers
    pub users: Vec<String>,
    /// ChainId for the target network
    pub chain_id: ChainId,
    /// Whether to allow new validators to join the set after genesis
    #[serde(default)]
    pub allow_new_validators: bool,
    /// Duration of an epoch
    pub epoch_duration_secs: u64,
    pub is_test: bool,
    /// Minimum stake to be in the validator set
    pub min_stake: u64,
    /// Minimum number of votes to consider a proposal valid.
    pub min_voting_threshold: u128,
    /// Maximum stake to be in the validator set
    pub max_stake: u64,
    /// Minimum number of seconds to lockup staked coins
    pub recurring_lockup_duration_secs: u64,
    /// Required amount of stake to create proposals.
    pub required_proposer_stake: u64,
    /// Percentage of stake given out as rewards a year (0-100%).
    pub rewards_apy_percentage: u64,
    /// Voting duration for a proposal in seconds.
    pub voting_duration_secs: u64,
    /// % of current epoch's total voting power that can be added in this epoch.
    pub voting_power_increase_limit: u64,
}

impl Layout {
    /// Read the layout from a YAML file on disk
    pub fn from_disk(path: &Path) -> anyhow::Result<Self> {
        let mut file = File::open(&path).map_err(|e| {
            anyhow::Error::msg(format!("Failed to open file {}, {}", path.display(), e))
        })?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| {
            anyhow::Error::msg(format!("Failed to read file {}, {}", path.display(), e))
        })?;

        Ok(serde_yaml::from_str(&contents)?)
    }
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            root_key: None,
            users: vec![],
            chain_id: ChainId::test(),
            allow_new_validators: false,
            epoch_duration_secs: 7_200,
            is_test: true,
            min_stake: 100_000_000_000_000,
            min_voting_threshold: 100_000_000_000_000,
            max_stake: 100_000_000_000_000_000,
            recurring_lockup_duration_secs: 86_400,
            required_proposer_stake: 100_000_000_000_000,
            rewards_apy_percentage: 10,
            voting_duration_secs: 43_200,
            voting_power_increase_limit: 20,
        }
    }
}

/// A set of configuration needed to add a Validator to genesis
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorConfiguration {
    /// Account address
    pub owner_account_address: AccountAddress,
    /// Key used for signing transactions with the account
    pub owner_account_public_key: Ed25519PublicKey,
    pub operator_account_address: AccountAddress,
    pub operator_account_public_key: Ed25519PublicKey,
    pub voter_account_address: AccountAddress,
    pub voter_account_public_key: Ed25519PublicKey,
    /// Key used for signing in consensus
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consensus_public_key: Option<bls12381::PublicKey>,
    /// Corresponding proof of possession of consensus public key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_of_possession: Option<bls12381::ProofOfPossession>,
    /// Public key used for validator network identity (same as account address)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validator_network_public_key: Option<x25519::PublicKey>,
    /// Host for validator which can be an IP or a DNS name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validator_host: Option<HostAndPort>,
    /// Public key used for full node network identity (same as account address)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_node_network_public_key: Option<x25519::PublicKey>,
    /// Host for full node which can be an IP or a DNS name and is optional
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_node_host: Option<HostAndPort>,
    /// Stake amount for consensus
    pub stake_amount: u64,
    /// Commission percentage for validator
    pub commission_percentage: u64,
    /// Whether the validator should be joining the validator set during genesis.
    /// If set to false, the validator will be fully initialized but won't be added to the
    /// validator set.
    pub join_during_genesis: bool,
}

impl TryFrom<ValidatorConfiguration> for ValidatorWithCommissionRate {
    type Error = anyhow::Error;

    fn try_from(config: ValidatorConfiguration) -> Result<Self, Self::Error> {
        let validator_commission_percentage = config.commission_percentage;
        let join_during_genesis = config.join_during_genesis;
        Ok(ValidatorWithCommissionRate {
            validator: config.try_into()?,
            validator_commission_percentage,
            join_during_genesis,
        })
    }
}

impl TryFrom<ValidatorConfiguration> for Validator {
    type Error = anyhow::Error;

    fn try_from(config: ValidatorConfiguration) -> Result<Self, Self::Error> {
        let validator_addresses = if let Some(validator_host) = config.validator_host {
            if let Some(validator_network_public_key) = config.validator_network_public_key {
                vec![validator_host
                    .as_network_address(validator_network_public_key)
                    .unwrap()]
            } else {
                return Err(anyhow::Error::msg(
                    "Validator addresses specified, but not validator network key",
                ));
            }
        } else {
            vec![]
        };

        let full_node_addresses = if let Some(full_node_host) = config.full_node_host {
            if let Some(full_node_network_key) = config.full_node_network_public_key {
                vec![full_node_host
                    .as_network_address(full_node_network_key)
                    .unwrap()]
            } else {
                return Err(anyhow::Error::msg(
                    "Full node host specified, but not full node network key",
                ));
            }
        } else {
            vec![]
        };

        let auth_key = AuthenticationKey::ed25519(&config.owner_account_public_key);
        let derived_address = auth_key.derived_address();
        if config.owner_account_address != derived_address {
            return Err(anyhow::Error::msg(format!(
                "owner_account_address {} does not match account key derived one {}",
                config.owner_account_address, derived_address
            )));
        }

        let auth_key = AuthenticationKey::ed25519(&config.operator_account_public_key);
        let derived_address = auth_key.derived_address();
        if config.operator_account_address != derived_address {
            return Err(anyhow::Error::msg(format!(
                "operator_account_address {} does not match account key derived one {}",
                config.operator_account_address, derived_address
            )));
        }

        let auth_key = AuthenticationKey::ed25519(&config.voter_account_public_key);
        let derived_address = auth_key.derived_address();
        if config.voter_account_address != derived_address {
            return Err(anyhow::Error::msg(format!(
                "voter_account_address {} does not match account key derived one {}",
                config.voter_account_address, derived_address
            )));
        }

        let consensus_pubkey = if let Some(consensus_public_key) = config.consensus_public_key {
            consensus_public_key.to_bytes().to_vec()
        } else {
            vec![]
        };
        let proof_of_possession = if let Some(pop) = config.proof_of_possession {
            pop.to_bytes().to_vec()
        } else {
            vec![]
        };

        Ok(Validator {
            owner_address: config.owner_account_address,
            operator_address: config.operator_account_address,
            voter_address: config.voter_account_address,
            consensus_pubkey,
            proof_of_possession,
            network_addresses: bcs::to_bytes(&validator_addresses).unwrap(),
            full_node_network_addresses: bcs::to_bytes(&full_node_addresses).unwrap(),
            stake_amount: config.stake_amount,
        })
    }
}

const LOCALHOST: &str = "localhost";

/// Combined Host (DnsName or IP) and port
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostAndPort {
    pub host: DnsName,
    pub port: u16,
}

impl HostAndPort {
    pub fn local(port: u16) -> anyhow::Result<HostAndPort> {
        Ok(HostAndPort {
            host: DnsName::try_from(LOCALHOST.to_string())?,
            port,
        })
    }

    pub fn as_network_address(&self, key: x25519::PublicKey) -> anyhow::Result<NetworkAddress> {
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

        Ok(NetworkAddress::try_from(vec![
            host_protocol,
            port_protocol,
            noise_protocol,
            handshake_protocol,
        ])?)
    }
}

impl TryFrom<&NetworkAddress> for HostAndPort {
    type Error = anyhow::Error;

    fn try_from(address: &NetworkAddress) -> Result<Self, Self::Error> {
        let socket_addr = address.to_socket_addrs()?.next().unwrap();
        HostAndPort::from_str(&socket_addr.to_string())
    }
}

impl FromStr for HostAndPort {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split(':').collect();
        if parts.len() != 2 {
            Err(anyhow::Error::msg(
                "Invalid host and port, must be of the form 'host:port` e.g. '127.0.0.1:6180'",
            ))
        } else {
            let host = DnsName::from_str(*parts.first().unwrap())?;
            let port = u16::from_str(parts.get(1).unwrap())?;
            Ok(HostAndPort { host, port })
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OwnerConfiguration {
    pub owner_account_address: AccountAddress,
    pub owner_account_public_key: Ed25519PublicKey,
    pub voter_account_address: AccountAddress,
    pub voter_account_public_key: Ed25519PublicKey,
    pub operator_account_address: AccountAddress,
    pub operator_account_public_key: Ed25519PublicKey,
    pub stake_amount: u64,
    pub commission_percentage: u64,
    pub join_during_genesis: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OperatorConfiguration {
    pub operator_account_address: AccountAddress,
    pub operator_account_public_key: Ed25519PublicKey,
    pub consensus_public_key: bls12381::PublicKey,
    pub consensus_proof_of_possession: bls12381::ProofOfPossession,
    pub validator_network_public_key: x25519::PublicKey,
    pub validator_host: HostAndPort,
    pub full_node_network_public_key: Option<x25519::PublicKey>,
    pub full_node_host: Option<HostAndPort>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StringOwnerConfiguration {
    pub owner_account_address: Option<String>,
    pub owner_account_public_key: Option<String>,
    pub voter_account_address: Option<String>,
    pub voter_account_public_key: Option<String>,
    pub operator_account_address: Option<String>,
    pub operator_account_public_key: Option<String>,
    pub stake_amount: Option<String>,
    pub commission_percentage: Option<String>,
    pub join_during_genesis: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StringOperatorConfiguration {
    pub operator_account_address: Option<String>,
    pub operator_account_public_key: Option<String>,
    pub consensus_public_key: Option<String>,
    pub consensus_proof_of_possession: Option<String>,
    pub validator_network_public_key: Option<String>,
    pub validator_host: HostAndPort,
    pub full_node_network_public_key: Option<String>,
    pub full_node_host: Option<HostAndPort>,
}
