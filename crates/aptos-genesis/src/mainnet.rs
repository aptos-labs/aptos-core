// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{builder::GenesisConfiguration, config::ValidatorConfiguration};
use aptos_config::config::{
    RocksdbConfigs, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
    TARGET_SNAPSHOT_SIZE,
};
use aptos_temppath::TempPath;
use aptos_types::{chain_id::ChainId, transaction::Transaction, waypoint::Waypoint};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use framework::ReleaseBundle;
use storage_interface::DbReaderWriter;
use vm_genesis::{AccountMap, EmployeeAccountMap, ValidatorWithCommissionRate};

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
    accounts: Vec<AccountMap>,
    /// Employee vesting configurations.
    employee_vesting_accounts: Vec<EmployeeAccountMap>,
    /// Set of configurations for validators who will be joining the genesis validator set.
    validators: Vec<ValidatorWithCommissionRate>,
}

impl MainnetGenesisInfo {
    pub fn new(
        chain_id: ChainId,
        accounts: Vec<AccountMap>,
        employee_vesting_accounts: Vec<EmployeeAccountMap>,
        validators: Vec<ValidatorConfiguration>,
        framework: ReleaseBundle,
        genesis_config: &GenesisConfiguration,
    ) -> anyhow::Result<MainnetGenesisInfo> {
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
        vm_genesis::encode_aptos_mainnet_genesis_transaction(
            &self.accounts,
            &self.employee_vesting_accounts,
            &self.validators,
            &self.framework,
            self.chain_id,
            &vm_genesis::GenesisConfiguration {
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
            },
        )
    }

    pub fn generate_waypoint(&mut self) -> anyhow::Result<Waypoint> {
        let genesis = self.get_genesis();
        let path = TempPath::new();
        let aptosdb = AptosDB::open(
            &path,
            false,
            NO_OP_STORAGE_PRUNER_CONFIG,
            RocksdbConfigs::default(),
            false,
            TARGET_SNAPSHOT_SIZE,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
        )?;
        let db_rw = DbReaderWriter::new(aptosdb);
        executor::db_bootstrapper::generate_waypoint::<AptosVM>(&db_rw, genesis)
    }
}
