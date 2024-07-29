// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{add_accounts_impl, PipelineConfig};
use aptos_config::{
    config::{
        PrunerConfig, RocksdbConfigs, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
    },
    utils::get_genesis_txn,
};
use aptos_db::AptosDB;
use aptos_executor::{
    block_executor::TransactionBlockExecutor,
    db_bootstrapper::{generate_waypoint, maybe_bootstrap},
};
use aptos_storage_interface::DbReaderWriter;
use aptos_types::on_chain_config::Features;
use aptos_vm::AptosVM;
use std::{fs, path::Path};

pub fn create_db_with_accounts<V>(
    num_accounts: usize,
    init_account_balance: u64,
    block_size: usize,
    db_dir: impl AsRef<Path>,
    storage_pruner_config: PrunerConfig,
    verify_sequence_numbers: bool,
    enable_storage_sharding: bool,
    pipeline_config: PipelineConfig,
    init_features: Features,
) where
    V: TransactionBlockExecutor + 'static,
{
    println!("Initializing...");

    if db_dir.as_ref().exists() {
        panic!("data-dir exists already.");
    }
    // create if not exists
    fs::create_dir_all(db_dir.as_ref()).unwrap();

    bootstrap_with_genesis(&db_dir, enable_storage_sharding, init_features.clone());

    println!(
        "Finished empty DB creation, DB dir: {}. Creating accounts now...",
        db_dir.as_ref().display()
    );

    add_accounts_impl::<V>(
        num_accounts,
        init_account_balance,
        block_size,
        &db_dir,
        &db_dir,
        storage_pruner_config,
        verify_sequence_numbers,
        enable_storage_sharding,
        pipeline_config,
        init_features,
    );
}

fn bootstrap_with_genesis(
    db_dir: impl AsRef<Path>,
    enable_storage_sharding: bool,
    init_features: Features,
) {
    let (config, _genesis_key) =
        aptos_genesis::test_utils::test_config_with_custom_features(init_features);

    let mut rocksdb_configs = RocksdbConfigs::default();
    rocksdb_configs.state_merkle_db_config.max_open_files = -1;
    rocksdb_configs.enable_storage_sharding = enable_storage_sharding;
    let (_db, db_rw) = DbReaderWriter::wrap(
        AptosDB::open(
            StorageDirPaths::from_path(db_dir),
            false, /* readonly */
            NO_OP_STORAGE_PRUNER_CONFIG,
            rocksdb_configs,
            false, /* indexer */
            BUFFERED_STATE_TARGET_ITEMS,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
        )
        .expect("DB should open."),
    );

    // Bootstrap db with genesis
    let waypoint = generate_waypoint::<AptosVM>(&db_rw, get_genesis_txn(&config).unwrap()).unwrap();
    maybe_bootstrap::<AptosVM>(&db_rw, get_genesis_txn(&config).unwrap(), waypoint).unwrap();
}
