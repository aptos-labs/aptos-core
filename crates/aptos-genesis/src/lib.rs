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
    chain_id: ChainId,
    root_key: Ed25519PublicKey,
    validators: Vec<Validator>,
    /// Compiled bytecode of framework modules
    modules: Vec<Vec<u8>>,
    min_price_per_gas_unit: u64,
    genesis: Option<Transaction>,
}

impl GenesisInfo {
    pub fn new(
        chain_id: ChainId,
        root_key: Ed25519PublicKey,
        configs: Vec<ValidatorConfiguration>,
        modules: Vec<Vec<u8>>,
        min_price_per_gas_unit: u64,
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
