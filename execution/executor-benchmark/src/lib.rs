// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod account_generator;
pub mod db_generator;
pub mod pipeline;
pub mod state_committer;
pub mod transaction_committer;
pub mod transaction_executor;
pub mod transaction_generator;

use crate::{
    transaction_committer::TransactionCommitter, transaction_executor::TransactionExecutor,
    transaction_generator::TransactionGenerator,
};
use aptos_config::config::{
    NodeConfig, RocksdbConfig, StoragePrunerConfig, NO_OP_STORAGE_PRUNER_CONFIG,
};

use crate::{pipeline::Pipeline, state_committer::StateCommitter};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use executor::block_executor::BlockExecutor;
use std::{fs, path::Path};
use storage_interface::DbReaderWriter;

pub fn init_db_and_executor(config: &NodeConfig) -> (DbReaderWriter, BlockExecutor<AptosVM>) {
    let db = DbReaderWriter::new(
        AptosDB::open(
            &config.storage.dir(),
            false,                       /* readonly */
            NO_OP_STORAGE_PRUNER_CONFIG, /* pruner */
            RocksdbConfig::default(),
        )
        .expect("DB should open."),
    );

    let executor = BlockExecutor::new(db.clone());

    (db, executor)
}

/// Runs the benchmark with given parameters.
pub fn run_benchmark(
    block_size: usize,
    num_transfer_blocks: usize,
    source_dir: impl AsRef<Path>,
    checkpoint_dir: impl AsRef<Path>,
    verify_sequence_numbers: bool,
    pruner_config: StoragePrunerConfig,
) {
    // Create rocksdb checkpoint.
    if checkpoint_dir.as_ref().exists() {
        fs::remove_dir_all(checkpoint_dir.as_ref()).unwrap_or(());
    }
    std::fs::create_dir_all(checkpoint_dir.as_ref()).unwrap();

    AptosDB::open(
        &source_dir,
        true,                        /* readonly */
        NO_OP_STORAGE_PRUNER_CONFIG, /* pruner */
        RocksdbConfig::default(),
    )
    .expect("db open failure.")
    .create_checkpoint(checkpoint_dir.as_ref())
    .expect("db checkpoint creation fails.");

    let (mut config, _genesis_key) = aptos_genesis::test_utils::test_config();
    config.storage.dir = checkpoint_dir.as_ref().to_path_buf();
    config.storage.storage_pruner_config = pruner_config;

    let (db, executor) = init_db_and_executor(&config);
    let version = db.reader.get_latest_version().unwrap();

    let (pipeline, block_sender) = Pipeline::new(db.clone(), executor, version);

    let mut generator =
        TransactionGenerator::new_with_existing_db(block_sender, source_dir, version);
    generator.run_transfer(block_size, num_transfer_blocks);
    generator.drop_sender();
    pipeline.join();

    if verify_sequence_numbers {
        generator.verify_sequence_numbers(db.reader);
    }
}

#[cfg(test)]
mod tests {
    use aptos_config::config::NO_OP_STORAGE_PRUNER_CONFIG;
    use aptos_temppath::TempPath;

    #[test]
    fn test_benchmark() {
        let storage_dir = TempPath::new();
        let checkpoint_dir = TempPath::new();

        crate::db_generator::run(
            25,    /* num_accounts */
            10000, /* init_account_balance */
            5,     /* block_size */
            storage_dir.as_ref(),
            NO_OP_STORAGE_PRUNER_CONFIG, /* prune_window */
            true,
        );

        super::run_benchmark(
            5, /* block_size */
            5, /* num_transfer_blocks */
            storage_dir.as_ref(),
            checkpoint_dir,
            true,
            NO_OP_STORAGE_PRUNER_CONFIG,
        );
    }
}
