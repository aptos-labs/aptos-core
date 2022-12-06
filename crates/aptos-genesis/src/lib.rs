// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

pub mod builder;
pub mod config;
pub mod keys;
pub mod mainnet;

#[cfg(any(test, feature = "testing"))]
pub mod test_utils;

use crate::{builder::GenesisConfiguration, config::ValidatorConfiguration};
use aptos_config::config::{
    RocksdbConfigs, BUFFERED_STATE_TARGET_ITEMS, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_temppath::TempPath;
use aptos_types::{
    chain_id::ChainId,
    on_chain_config::{GasScheduleV2, OnChainConsensusConfig},
    transaction::Transaction,
    waypoint::Waypoint,
};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use framework::ReleaseBundle;
use std::convert::TryInto;
use storage_interface::DbReaderWriter;
use vm_genesis::{
    GenesisEmployeeVestingConfiguration, GenesisGovernanceConfiguration,
    GenesisRewardsConfiguration, GenesisStakingConfiguration, Validator,
};

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
    pub staking: GenesisStakingConfiguration,
    pub rewards: GenesisRewardsConfiguration,
    pub governance: GenesisGovernanceConfiguration,
    pub employee_vesting: GenesisEmployeeVestingConfiguration,

    pub consensus_config: OnChainConsensusConfig,
    pub gas_schedule: GasScheduleV2,
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
            staking: genesis_config.staking,
            rewards: genesis_config.rewards,
            governance: genesis_config.governance,
            employee_vesting: genesis_config.employee_vesting,
            consensus_config: genesis_config.consensus_config.clone(),
            gas_schedule: genesis_config.gas_schedule.clone(),
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
        vm_genesis::encode_genesis_transaction(
            self.root_key.clone(),
            &self.validators,
            &self.framework,
            self.chain_id,
            &vm_genesis::GenesisConfiguration {
                allow_new_validators: self.allow_new_validators,
                epoch_duration_secs: self.epoch_duration_secs,
                is_test: true,
                staking: self.staking,
                rewards: self.rewards,
                governance: self.governance,
                employee_vesting: self.employee_vesting,
            },
            &self.consensus_config,
            &self.gas_schedule,
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
