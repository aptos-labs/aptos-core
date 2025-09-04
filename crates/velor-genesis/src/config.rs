// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_config::config::HANDSHAKE_VERSION;
use velor_crypto::{bls12381, ed25519::Ed25519PublicKey, x25519};
use velor_types::{
    account_address::{AccountAddress, AccountAddressWithChecks},
    chain_id::ChainId,
    jwks::patch::IssuerJWK,
    keyless::Groth16VerificationKey,
    network_address::{DnsName, NetworkAddress, Protocol},
    on_chain_config::{OnChainConsensusConfig, OnChainExecutionConfig, OnChainJWKConsensusConfig},
    transaction::authenticator::AuthenticationKey,
};
use velor_vm_genesis::{AccountBalance, EmployeePool, Validator, ValidatorWithCommissionRate};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::{BTreeMap, HashSet},
    convert::TryFrom,
    fs::File,
    io::Read,
    net::{Ipv4Addr, Ipv6Addr, ToSocketAddrs},
    path::Path,
    str::FromStr,
};

/// Template for setting up Github for Genesis
///
#[derive(Debug, Deserialize, Serialize)]
pub struct Layout {
    /// Root key for the blockchain only for test chains
    #[serde(default)]
    pub root_key: Option<Ed25519PublicKey>,
    /// List of usernames or identifiers
    pub users: Vec<String>,
    /// ChainId for the target network
    pub chain_id: ChainId,
    /// Whether to allow new validators to join the set after genesis
    ///
    /// Ignored for mainnet
    #[serde(default)]
    pub allow_new_validators: bool,
    /// Duration of an epoch
    pub epoch_duration_secs: u64,
    /// Whether this is a test network or not
    ///
    /// Ignored for mainnet
    #[serde(default)]
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
    /// Total supply of coins
    pub total_supply: Option<u64>,
    /// Timestamp (in seconds) when employee vesting starts.
    pub employee_vesting_start: Option<u64>,
    /// Duration of each vesting period (in seconds).
    pub employee_vesting_period_duration: Option<u64>,
    /// Onchain Consensus Config
    #[serde(default = "OnChainConsensusConfig::default_for_genesis")]
    pub on_chain_consensus_config: OnChainConsensusConfig,
    /// Onchain Execution Config
    #[serde(default = "OnChainExecutionConfig::default_for_genesis")]
    pub on_chain_execution_config: OnChainExecutionConfig,

    /// An optional JWK consensus config to use, instead of `default_for_genesis()`.
    #[serde(default)]
    pub jwk_consensus_config_override: Option<OnChainJWKConsensusConfig>,

    /// JWKs to patch in genesis.
    #[serde(default)]
    pub initial_jwks: Vec<IssuerJWK>,

    /// Keyless Groth16 verification key to install in genesis.
    #[serde(default)]
    pub keyless_groth16_vk_override: Option<Groth16VerificationKey>,
}

impl Layout {
    /// Read the layout from a YAML file on disk
    pub fn from_disk(path: &Path) -> anyhow::Result<Self> {
        let mut file = File::open(path).map_err(|e| {
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
            total_supply: None,
            employee_vesting_start: Some(1663456089),
            employee_vesting_period_duration: Some(5 * 60), // 5 minutes
            on_chain_consensus_config: OnChainConsensusConfig::default(),
            on_chain_execution_config: OnChainExecutionConfig::default_for_genesis(),
            jwk_consensus_config_override: None,
            initial_jwks: vec![],
            keyless_groth16_vk_override: None,
        }
    }
}

/// A set of configuration needed to add a Validator to genesis
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorConfiguration {
    /// Account address
    pub owner_account_address: AccountAddressWithChecks,
    /// Key used for signing transactions with the account
    pub owner_account_public_key: Ed25519PublicKey,
    pub operator_account_address: AccountAddressWithChecks,
    pub operator_account_public_key: Ed25519PublicKey,
    pub voter_account_address: AccountAddressWithChecks,
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
        let account_address = auth_key.account_address();
        let owner_address = AccountAddress::from(config.owner_account_address);
        if owner_address != account_address {
            return Err(anyhow::Error::msg(format!(
                "owner_account_address {} does not match account key derived one {}",
                owner_address, account_address
            )));
        }

        let auth_key = AuthenticationKey::ed25519(&config.operator_account_public_key);
        let account_address = auth_key.account_address();
        let operator_address = AccountAddress::from(config.operator_account_address);
        if operator_address != account_address {
            return Err(anyhow::Error::msg(format!(
                "operator_account_address {} does not match account key derived one {}",
                operator_address, account_address
            )));
        }

        let auth_key = AuthenticationKey::ed25519(&config.voter_account_public_key);
        let account_address = auth_key.account_address();
        let voter_address = AccountAddress::from(config.voter_account_address);
        if voter_address != account_address {
            return Err(anyhow::Error::msg(format!(
                "voter_account_address {} does not match account key derived one {}",
                voter_address, account_address
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
            owner_address,
            operator_address,
            voter_address,
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
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
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
            let host_str = *parts.first().unwrap();
            if host_str.trim().is_empty() {
                Err(anyhow::Error::msg("Invalid host, host is empty"))
            } else {
                let host = DnsName::from_str(host_str)?;
                let port = u16::from_str(parts.get(1).unwrap())?;
                Ok(HostAndPort { host, port })
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OwnerConfiguration {
    pub owner_account_address: AccountAddressWithChecks,
    pub owner_account_public_key: Ed25519PublicKey,
    pub voter_account_address: AccountAddressWithChecks,
    pub voter_account_public_key: Ed25519PublicKey,
    pub operator_account_address: AccountAddressWithChecks,
    pub operator_account_public_key: Ed25519PublicKey,
    pub stake_amount: u64,
    pub commission_percentage: u64,
    pub join_during_genesis: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OperatorConfiguration {
    pub operator_account_address: AccountAddressWithChecks,
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

#[derive(Debug, Clone)]
pub struct AccountBalanceMap {
    pub account_balances: Vec<BTreeMap<AccountAddress, u64>>,
}

impl Serialize for AccountBalanceMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.account_balances.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AccountBalanceMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let account_balances =
            <Vec<BTreeMap<AccountAddressWithChecks, u64>>>::deserialize(deserializer)?;
        let account_balances = account_balances
            .into_iter()
            .map(|map| {
                map.into_iter()
                    .map(|(addr, balance)| (addr.into(), balance))
                    .collect()
            })
            .collect();
        Ok(AccountBalanceMap { account_balances })
    }
}

impl TryFrom<Vec<AccountBalance>> for AccountBalanceMap {
    type Error = anyhow::Error;

    fn try_from(balances: Vec<AccountBalance>) -> Result<Self, Self::Error> {
        let mut accounts = HashSet::new();
        let mut vector = vec![];
        let mut errors = vec![];
        for balance in balances {
            let mut map = BTreeMap::new();
            map.insert(balance.account_address, balance.balance);
            if !accounts.insert(balance.account_address) {
                errors.push(anyhow::anyhow!(
                    "An account was duplicated {}",
                    balance.account_address
                ));
            }

            vector.push(map);
        }

        if !errors.is_empty() {
            Err(anyhow::anyhow!(
                "There are duplicated accounts: {:?}",
                errors
            ))
        } else {
            Ok(AccountBalanceMap {
                account_balances: vector,
            })
        }
    }
}

impl TryFrom<AccountBalanceMap> for Vec<AccountBalance> {
    type Error = anyhow::Error;

    fn try_from(balance_map: AccountBalanceMap) -> Result<Self, Self::Error> {
        let mut accounts = HashSet::new();
        let mut balances = vec![];
        let mut errors = vec![];
        for (i, balance_entry) in balance_map.account_balances.iter().enumerate() {
            let (account_address, balance) = balance_entry
                .iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("No account in entry #{}", i))?;

            if !accounts.insert(*account_address) {
                errors.push(anyhow::anyhow!(
                    "An account was duplicated {} in the balances at entry #{}",
                    account_address,
                    i
                ));
            }

            balances.push(AccountBalance {
                account_address: *account_address,
                balance: *balance,
            });
        }

        if !errors.is_empty() {
            Err(anyhow::anyhow!(
                "There are duplicated accounts: {:?}",
                errors
            ))
        } else {
            Ok(balances)
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmployeePoolMap {
    pub inner: Vec<EmployeePoolConfig>,
}

impl Serialize for EmployeePoolMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EmployeePoolMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner = <Vec<EmployeePoolConfig>>::deserialize(deserializer)?;
        Ok(EmployeePoolMap { inner })
    }
}

impl TryFrom<EmployeePoolMap> for Vec<EmployeePool> {
    type Error = anyhow::Error;

    fn try_from(map: EmployeePoolMap) -> Result<Self, Self::Error> {
        let mut vesting_schedule_numbers: Option<(Vec<u64>, u64)> = None;
        let mut beneficiary_resetter: Option<AccountAddress> = None;

        let mut employee_accounts = HashSet::new();
        let mut pools = vec![];
        let mut errors = vec![];
        for (i, pool) in map.inner.into_iter().enumerate() {
            // Check for duplicate employee accounts
            for (j, employee_account) in pool.accounts.iter().enumerate() {
                if !employee_accounts.insert(*employee_account) {
                    errors.push(anyhow::anyhow!(
                        "Employee account #{} {} duplicated in employee pool #{}",
                        j,
                        employee_account,
                        i
                    ));
                }
            }

            // Check vesting schedule adds up properly, we only have to check once, then check they are all the same
            if let Some((numerators, denominator)) = vesting_schedule_numbers.as_ref() {
                if numerators != &pool.vesting_schedule_numerators {
                    errors.push(anyhow::anyhow!("Numerators are not the same on every pool in employee pool #{}.  Expected: {:?}, got {:?}", i, numerators, pool.vesting_schedule_numerators));
                }

                if denominator != &pool.vesting_schedule_denominator {
                    errors.push(anyhow::anyhow!("Denominator are not the same on every pool in employee pool #{}.  Expected: {:?}, got {:?}", i, denominator, pool.vesting_schedule_denominator));
                }
            } else {
                let mut numerators = 0;
                let denominator = pool.vesting_schedule_denominator;
                let mut last_numerator = 0;
                for numerator in pool.vesting_schedule_numerators.iter() {
                    numerators += *numerator;
                    last_numerator = *numerator;
                }

                if denominator == 0 {
                    errors.push(anyhow::anyhow!(
                        "Denominator can't be 0 for employee pool #{}",
                        i
                    ));
                }

                if numerators > denominator {
                    errors.push(anyhow::anyhow!(
                        "Numerators {} add up over the denominator {} for employee pool #{}",
                        numerators,
                        denominator,
                        i
                    ));
                } else if (denominator - numerators) % last_numerator != 0 {
                    errors.push(anyhow::anyhow!("Numerators don't add up to the denominator {} (with the last one {} being repeated for employee pool #{}", denominator, last_numerator, i));
                }

                vesting_schedule_numbers = Some((
                    pool.vesting_schedule_numerators.clone(),
                    pool.vesting_schedule_denominator,
                ))
            }

            // I'm going to assume no one wants to pay more than 50% of their rewards away
            if pool.validator.commission_percentage > 50 {
                errors.push(anyhow::anyhow!(
                    "Commission percentage is larger than 50% ({}%) for employee pool #{}",
                    pool.validator.commission_percentage,
                    i
                ));
            }

            // If joining during genesis, it needs all the setup
            if pool.validator.join_during_genesis {
                if pool.validator.consensus_public_key.is_none() {
                    errors.push(anyhow::anyhow!("Employee pool #{} is setup to join during genesis but missing a consensus public key", i));
                }
                if pool.validator.proof_of_possession.is_none() {
                    errors.push(anyhow::anyhow!("Employee pool #{} is setup to join during genesis but missing a proof of possession", i));
                }
                if pool.validator.validator_host.is_none() {
                    errors.push(anyhow::anyhow!(
                        "Employee pool #{} is setup to join during genesis but missing a validator host",
                        i
                    ));
                }
                if pool.validator.validator_network_public_key.is_none() {
                    errors.push(anyhow::anyhow!("Employee pool #{} is setup to join during genesis but missing a validator network public key", i));
                }
                if pool.validator.stake_amount < 100000000000000 {
                    errors.push(anyhow::anyhow!(
                        "Employee pool #{} is setup to join during genesis but has a low stake amount {} < 1000000 APT",
                        i,
                        pool.validator.stake_amount
                    ));
                }
            }

            let pool_beneficiary_resetter = AccountAddress::from(pool.beneficiary_resetter);
            if let Some(beneficiary_resetter) = beneficiary_resetter {
                if beneficiary_resetter != pool_beneficiary_resetter {
                    errors.push(anyhow::anyhow!(
                        "Employee pool #{} has the wrong beneficiary resetter.  Found {}, should have {}",
                        i,
                        pool.beneficiary_resetter,
                        beneficiary_resetter
                    ));
                }
            } else {
                beneficiary_resetter = Some(pool_beneficiary_resetter);
            }

            pools.push(EmployeePool::try_from(pool)?);
        }

        if errors.is_empty() {
            Ok(pools)
        } else {
            Err(anyhow::anyhow!(
                "Failed with the following errors: {:?}",
                errors
            ))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeePoolConfig {
    pub accounts: Vec<AccountAddressWithChecks>,
    pub validator: ValidatorConfiguration,
    pub vesting_schedule_numerators: Vec<u64>,
    pub vesting_schedule_denominator: u64,
    pub beneficiary_resetter: AccountAddressWithChecks,
}

impl TryFrom<EmployeePoolConfig> for EmployeePool {
    type Error = anyhow::Error;

    fn try_from(pool: EmployeePoolConfig) -> Result<Self, Self::Error> {
        let validator_commission_percentage = pool.validator.commission_percentage;
        let join_during_genesis = pool.validator.join_during_genesis;
        let validator = Validator::try_from(pool.validator)?;
        Ok(EmployeePool {
            accounts: pool
                .accounts
                .into_iter()
                .map(|inner| inner.into())
                .collect(),
            validator: ValidatorWithCommissionRate {
                validator,
                validator_commission_percentage,
                join_during_genesis,
            },
            vesting_schedule_numerators: pool.vesting_schedule_numerators,
            vesting_schedule_denominator: pool.vesting_schedule_denominator,
            beneficiary_resetter: pool.beneficiary_resetter.into(),
        })
    }
}
