// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{builder::GenesisConfiguration, config::ValidatorConfiguration};
use aptos_config::config::{
    RocksdbConfigs, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS,
    DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_db::AptosDB;
use aptos_framework::ReleaseBundle;
use aptos_storage_interface::DbReaderWriter;
use aptos_temppath::TempPath;
use aptos_types::{
    chain_id::ChainId,
    on_chain_config::{Features, OnChainJWKConsensusConfig, OnChainRandomnessConfig},
    transaction::Transaction,
    waypoint::Waypoint,
};
use aptos_vm::aptos_vm::AptosVMBlockExecutor;
use aptos_vm_genesis::{AccountBalance, EmployeePool, ValidatorWithCommissionRate};

/// Holder object for all pieces needed to generate a genesis transaction
#[derive(Clone)]
pub struct MainnetGenesisInfo {
    /// ChainId for identifying the network
    chain_id: ChainId,
    /// Released framework packages
    framework: ReleaseBundle,
    /// The genesis transaction, once it's been generated
    genesis: Option<Transaction>,

    /// Duration of an epoch
    pub epoch_duration_secs: u64,
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

    // MAINNET SPECIFIC FIELDS.
    /// Initial accounts and balances.
    accounts: Vec<AccountBalance>,
    /// Employee vesting configurations.
    employee_vesting_accounts: Vec<EmployeePool>,
    /// Set of configurations for validators who will be joining the genesis validator set.
    validators: Vec<ValidatorWithCommissionRate>,
    /// Timestamp (in seconds) when employee vesting starts.
    employee_vesting_start: u64,
    /// Duration of each vesting period (in seconds).
    employee_vesting_period_duration: u64,
    /// An optional feature vec to replace the default one.
    initial_features_override: Option<Features>,
    /// An optional randomness config to replace `OnChainRandomnessConfig::default_for_genesis()`.
    randomness_config_override: Option<OnChainRandomnessConfig>,
    /// An optional feature vec to replace `OnChainJWKConsensusConfig::default_for_genesis()`.
    jwk_consensus_config_override: Option<OnChainJWKConsensusConfig>,
}

impl MainnetGenesisInfo {
    pub fn new(
        chain_id: ChainId,
        accounts: Vec<AccountBalance>,
        employee_vesting_accounts: Vec<EmployeePool>,
        validators: Vec<ValidatorConfiguration>,
        framework: ReleaseBundle,
        genesis_config: &GenesisConfiguration,
    ) -> anyhow::Result<MainnetGenesisInfo> {
        let employee_vesting_start = genesis_config
            .employee_vesting_start
            .expect("Employee vesting start time (in secs) needs to be provided");
        let employee_vesting_period_duration = genesis_config
            .employee_vesting_period_duration
            .expect("Employee vesting period duration (in secs) needs to be provided");

        Ok(MainnetGenesisInfo {
            chain_id,
            accounts,
            employee_vesting_accounts,
            validators: validators
                .into_iter()
                .map(|v| ValidatorWithCommissionRate::try_from(v).unwrap())
                .collect(),
            framework,
            genesis: None,
            epoch_duration_secs: genesis_config.epoch_duration_secs,
            min_stake: genesis_config.min_stake,
            min_voting_threshold: genesis_config.min_voting_threshold,
            max_stake: genesis_config.max_stake,
            recurring_lockup_duration_secs: genesis_config.recurring_lockup_duration_secs,
            required_proposer_stake: genesis_config.required_proposer_stake,
            rewards_apy_percentage: genesis_config.rewards_apy_percentage,
            voting_duration_secs: genesis_config.voting_duration_secs,
            voting_power_increase_limit: genesis_config.voting_power_increase_limit,
            employee_vesting_start,
            employee_vesting_period_duration,
            initial_features_override: genesis_config.initial_features_override.clone(),
            randomness_config_override: genesis_config.randomness_config_override.clone(),
            jwk_consensus_config_override: genesis_config.jwk_consensus_config_override.clone(),
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
        aptos_vm_genesis::encode_aptos_mainnet_genesis_transaction(
            &self.accounts,
            &self.employee_vesting_accounts,
            &self.validators,
            &self.framework,
            self.chain_id,
            &aptos_vm_genesis::GenesisConfiguration {
                allow_new_validators: true,
                is_test: false,
                epoch_duration_secs: self.epoch_duration_secs,
                min_stake: self.min_stake,
                min_voting_threshold: self.min_voting_threshold,
                max_stake: self.max_stake,
                recurring_lockup_duration_secs: self.recurring_lockup_duration_secs,
                required_proposer_stake: self.required_proposer_stake,
                rewards_apy_percentage: self.rewards_apy_percentage,
                voting_duration_secs: self.voting_duration_secs,
                voting_power_increase_limit: self.voting_power_increase_limit,
                employee_vesting_start: self.employee_vesting_start,
                employee_vesting_period_duration: self.employee_vesting_period_duration,
                initial_features_override: self.initial_features_override.clone(),
                randomness_config_override: self.randomness_config_override.clone(),
                jwk_consensus_config_override: self.jwk_consensus_config_override.clone(),
                initial_jwks: vec![],
                keyless_groth16_vk: None,
            },
        )
    }

    pub fn generate_waypoint(&mut self) -> anyhow::Result<Waypoint> {
        let genesis = self.get_genesis();
        let path = TempPath::new();
        let aptosdb = AptosDB::open(
            StorageDirPaths::from_path(path),
            false,
            NO_OP_STORAGE_PRUNER_CONFIG,
            &RocksdbConfigs::default(),
            false, /* indexer */
            BUFFERED_STATE_TARGET_ITEMS,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            None,
        )?;
        let db_rw = DbReaderWriter::new(aptosdb);
        aptos_executor::db_bootstrapper::generate_waypoint::<AptosVMBlockExecutor>(&db_rw, genesis)
    }
}
