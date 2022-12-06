// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{builder::GenesisConfiguration, config::ValidatorConfiguration};
use aptos_config::config::{
    RocksdbConfigs, BUFFERED_STATE_TARGET_ITEMS, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_temppath::TempPath;
use aptos_types::{chain_id::ChainId, transaction::Transaction, waypoint::Waypoint};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use framework::ReleaseBundle;
use storage_interface::DbReaderWriter;
use vm_genesis::{
    AccountBalance, EmployeePool, GenesisEmployeeVestingConfiguration,
    GenesisGovernanceConfiguration, GenesisRewardsConfiguration, GenesisStakingConfiguration,
    ValidatorWithCommissionRate,
};

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
    pub staking: GenesisStakingConfiguration,
    pub rewards: GenesisRewardsConfiguration,
    pub governance: GenesisGovernanceConfiguration,
    pub employee_vesting: GenesisEmployeeVestingConfiguration,

    // MAINNET SPECIFIC FIELDS.
    /// Initial accounts and balances.
    accounts: Vec<AccountBalance>,
    /// Employee vesting configurations.
    employee_vesting_accounts: Vec<EmployeePool>,
    /// Set of configurations for validators who will be joining the genesis validator set.
    validators: Vec<ValidatorWithCommissionRate>,
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
            staking: genesis_config.staking,
            rewards: genesis_config.rewards,
            governance: genesis_config.governance,
            employee_vesting: genesis_config.employee_vesting,
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
                staking: self.staking,
                rewards: self.rewards,
                governance: self.governance,
                employee_vesting: self.employee_vesting,
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
            BUFFERED_STATE_TARGET_ITEMS,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
        )?;
        let db_rw = DbReaderWriter::new(aptosdb);
        executor::db_bootstrapper::generate_waypoint::<AptosVM>(&db_rw, genesis)
    }
}
