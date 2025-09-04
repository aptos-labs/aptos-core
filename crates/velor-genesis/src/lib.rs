// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

pub mod builder;
pub mod config;
pub mod keys;
pub mod mainnet;

#[cfg(any(test, feature = "testing"))]
pub mod test_utils;

use crate::{builder::GenesisConfiguration, config::ValidatorConfiguration};
use velor_config::config::{
    RocksdbConfigs, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS,
    DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
};
use velor_crypto::ed25519::Ed25519PublicKey;
use velor_db::VelorDB;
use velor_framework::ReleaseBundle;
use velor_storage_interface::DbReaderWriter;
use velor_temppath::TempPath;
use velor_types::{
    chain_id::ChainId,
    jwks::patch::IssuerJWK,
    keyless::Groth16VerificationKey,
    on_chain_config::{
        Features, GasScheduleV2, OnChainConsensusConfig, OnChainExecutionConfig,
        OnChainJWKConsensusConfig, OnChainRandomnessConfig,
    },
    transaction::Transaction,
    waypoint::Waypoint,
};
use velor_vm::velor_vm::VelorVMBlockExecutor;
use velor_vm_genesis::Validator;
use std::convert::TryInto;

/// Holder object for all pieces needed to generate a genesis transaction
#[derive(Clone)]
pub struct GenesisInfo {
    /// ChainId for identifying the network
    chain_id: ChainId,
    /// Key used for minting tokens
    root_key: Ed25519PublicKey,
    /// Set of configurations for validators on the network
    validators: Vec<Validator>,
    /// Released framework packages
    framework: ReleaseBundle,
    /// The genesis transaction, once it's been generated
    genesis: Option<Transaction>,

    /// Whether to allow new validators to join the set after genesis
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
    /// Percent of current epoch's total voting power that can be added in this epoch.
    pub voting_power_increase_limit: u64,

    pub consensus_config: OnChainConsensusConfig,
    pub execution_config: OnChainExecutionConfig,
    pub gas_schedule: GasScheduleV2,
    pub initial_features_override: Option<Features>,
    pub randomness_config_override: Option<OnChainRandomnessConfig>,
    pub jwk_consensus_config_override: Option<OnChainJWKConsensusConfig>,
    pub initial_jwks: Vec<IssuerJWK>,
    pub keyless_groth16_vk: Option<Groth16VerificationKey>,
}

impl GenesisInfo {
    pub fn new(
        chain_id: ChainId,
        root_key: Ed25519PublicKey,
        configs: Vec<ValidatorConfiguration>,
        framework: ReleaseBundle,
        genesis_config: &GenesisConfiguration,
    ) -> anyhow::Result<GenesisInfo> {
        let mut validators = Vec::new();

        for config in configs {
            validators.push(config.try_into()?)
        }

        Ok(GenesisInfo {
            chain_id,
            root_key,
            validators,
            framework,
            genesis: None,
            allow_new_validators: genesis_config.allow_new_validators,
            epoch_duration_secs: genesis_config.epoch_duration_secs,
            is_test: genesis_config.is_test,
            min_stake: genesis_config.min_stake,
            min_voting_threshold: genesis_config.min_voting_threshold,
            max_stake: genesis_config.max_stake,
            recurring_lockup_duration_secs: genesis_config.recurring_lockup_duration_secs,
            required_proposer_stake: genesis_config.required_proposer_stake,
            rewards_apy_percentage: genesis_config.rewards_apy_percentage,
            voting_duration_secs: genesis_config.voting_duration_secs,
            voting_power_increase_limit: genesis_config.voting_power_increase_limit,
            consensus_config: genesis_config.consensus_config.clone(),
            execution_config: genesis_config.execution_config.clone(),
            gas_schedule: genesis_config.gas_schedule.clone(),
            initial_features_override: genesis_config.initial_features_override.clone(),
            randomness_config_override: genesis_config.randomness_config_override.clone(),
            jwk_consensus_config_override: genesis_config.jwk_consensus_config_override.clone(),
            initial_jwks: genesis_config.initial_jwks.clone(),
            keyless_groth16_vk: genesis_config.keyless_groth16_vk.clone(),
        })
    }

    pub fn get_genesis(&mut self) -> &Transaction {
        if let Some(ref genesis) = self.genesis {
            genesis
        } else {
            self.genesis = Some(self.generate_genesis_txn());
            self.genesis.as_ref().unwrap()
        }
    }

    fn generate_genesis_txn(&self) -> Transaction {
        velor_vm_genesis::encode_genesis_transaction(
            self.root_key.clone(),
            &self.validators,
            &self.framework,
            self.chain_id,
            &velor_vm_genesis::GenesisConfiguration {
                allow_new_validators: self.allow_new_validators,
                epoch_duration_secs: self.epoch_duration_secs,
                is_test: true,
                min_stake: self.min_stake,
                min_voting_threshold: self.min_voting_threshold,
                max_stake: self.max_stake,
                recurring_lockup_duration_secs: self.recurring_lockup_duration_secs,
                required_proposer_stake: self.required_proposer_stake,
                rewards_apy_percentage: self.rewards_apy_percentage,
                voting_duration_secs: self.voting_duration_secs,
                voting_power_increase_limit: self.voting_power_increase_limit,
                employee_vesting_start: 1663456089,
                employee_vesting_period_duration: 5 * 60, // 5 minutes
                initial_features_override: self.initial_features_override.clone(),
                randomness_config_override: self.randomness_config_override.clone(),
                jwk_consensus_config_override: self.jwk_consensus_config_override.clone(),
                initial_jwks: self.initial_jwks.clone(),
                keyless_groth16_vk: self.keyless_groth16_vk.clone(),
            },
            &self.consensus_config,
            &self.execution_config,
            &self.gas_schedule,
        )
    }

    pub fn generate_waypoint(&mut self) -> anyhow::Result<Waypoint> {
        let genesis = self.get_genesis();
        let path = TempPath::new();
        let velordb = VelorDB::open(
            StorageDirPaths::from_path(path),
            false,
            NO_OP_STORAGE_PRUNER_CONFIG,
            RocksdbConfigs::default(),
            false, /* indexer */
            BUFFERED_STATE_TARGET_ITEMS,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            None,
        )?;
        let db_rw = DbReaderWriter::new(velordb);
        velor_executor::db_bootstrapper::generate_waypoint::<VelorVMBlockExecutor>(&db_rw, genesis)
    }
}
