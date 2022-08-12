// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

pub mod builder;
pub mod config;
pub mod keys;

#[cfg(any(test, feature = "testing"))]
pub mod test_utils;

use crate::config::ValidatorConfiguration;
use aptos_config::config::{RocksdbConfigs, NO_OP_STORAGE_PRUNER_CONFIG, TARGET_SNAPSHOT_SIZE};
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_temppath::TempPath;
use aptos_types::{chain_id::ChainId, transaction::Transaction, waypoint::Waypoint};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use std::convert::TryInto;
use storage_interface::DbReaderWriter;
use vm_genesis::Validator;

/// Holder object for all pieces needed to generate a genesis transaction
#[derive(Clone)]
pub struct GenesisInfo {
    /// ChainId for identifying the network
    chain_id: ChainId,
    /// Key used for minting tokens
    root_key: Ed25519PublicKey,
    /// Set of configurations for validators on the network
    validators: Vec<Validator>,
    /// Compiled bytecode of framework modules
    modules: Vec<Vec<u8>>,
    /// The genesis transaction, once it's been generated
    genesis: Option<Transaction>,

    /// Whether to allow new validators to join the set after genesis
    pub allow_new_validators: bool,
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
}

impl GenesisInfo {
    pub fn new(
        chain_id: ChainId,
        root_key: Ed25519PublicKey,
        configs: Vec<ValidatorConfiguration>,
        modules: Vec<Vec<u8>>,
        allow_new_validators: bool,
        epoch_duration_secs: u64,
        min_stake: u64,
        min_voting_threshold: u128,
        max_stake: u64,
        recurring_lockup_duration_secs: u64,
        required_proposer_stake: u64,
        rewards_apy_percentage: u64,
        voting_duration_secs: u64,
    ) -> anyhow::Result<GenesisInfo> {
        let mut validators = Vec::new();

        for config in configs {
            validators.push(config.try_into()?)
        }

        Ok(GenesisInfo {
            chain_id,
            root_key,
            validators,
            modules,
            genesis: None,
            allow_new_validators,
            epoch_duration_secs,
            min_stake,
            min_voting_threshold,
            max_stake,
            recurring_lockup_duration_secs,
            required_proposer_stake,
            rewards_apy_percentage,
            voting_duration_secs,
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
            &self.modules,
            self.chain_id,
            vm_genesis::GenesisConfiguration {
                allow_new_validators: self.allow_new_validators,
                epoch_duration_secs: self.epoch_duration_secs,
                min_stake: self.min_stake,
                min_voting_threshold: self.min_voting_threshold,
                max_stake: self.max_stake,
                recurring_lockup_duration_secs: self.recurring_lockup_duration_secs,
                required_proposer_stake: self.required_proposer_stake,
                rewards_apy_percentage: self.rewards_apy_percentage,
                voting_duration_secs: self.voting_duration_secs,
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
        )?;
        let db_rw = DbReaderWriter::new(aptosdb);
        executor::db_bootstrapper::generate_waypoint::<AptosVM>(&db_rw, genesis)
    }
}
