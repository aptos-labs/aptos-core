// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

pub mod builder;
pub mod config;
pub mod keys;

#[cfg(any(test, feature = "testing"))]
pub mod test_utils;

use crate::config::ValidatorConfiguration;
use aptos_config::config::{RocksdbConfig, NO_OP_STORAGE_PRUNER_CONFIG};
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
    min_price_per_gas_unit: u64,
    /// Whether to allow new validators to join the set after genesis
    pub allow_new_validators: bool,
    /// Minimum stake to be in the validator set
    pub min_stake: u64,
    /// Maximum stake to be in the validator set
    pub max_stake: u64,
    /// Minimum number of seconds to lockup staked coins
    pub min_lockup_duration_secs: u64,
    /// Maximum number of seconds to lockup staked coins
    pub max_lockup_duration_secs: u64,
    /// Duration of an epoch
    pub epoch_duration_secs: u64,
    /// Initial timestamp for genesis validators to be locked up
    pub initial_lockup_timestamp: u64,
    /// The genesis transaction, once it's been generated
    genesis: Option<Transaction>,
}

impl GenesisInfo {
    pub fn new(
        chain_id: ChainId,
        root_key: Ed25519PublicKey,
        configs: Vec<ValidatorConfiguration>,
        modules: Vec<Vec<u8>>,
        min_price_per_gas_unit: u64,
        allow_new_validators: bool,
        min_stake: u64,
        max_stake: u64,
        min_lockup_duration_secs: u64,
        max_lockup_duration_secs: u64,
        epoch_duration_secs: u64,
        initial_lockup_timestamp: u64,
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
            min_price_per_gas_unit,
            allow_new_validators,
            min_stake,
            max_stake,
            min_lockup_duration_secs,
            max_lockup_duration_secs,
            epoch_duration_secs,
            initial_lockup_timestamp,
            genesis: None,
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
            self.min_price_per_gas_unit,
            self.epoch_duration_secs,
            self.min_stake,
            self.max_stake,
            self.min_lockup_duration_secs,
            self.max_lockup_duration_secs,
            self.allow_new_validators,
        )
    }

    pub fn generate_waypoint(&mut self) -> anyhow::Result<Waypoint> {
        let genesis = self.get_genesis();
        let path = TempPath::new();
        let aptosdb = AptosDB::open(
            &path,
            false,
            NO_OP_STORAGE_PRUNER_CONFIG,
            RocksdbConfig::default(),
        )?;
        let db_rw = DbReaderWriter::new(aptosdb);
        executor::db_bootstrapper::generate_waypoint::<AptosVM>(&db_rw, genesis)
    }
}
