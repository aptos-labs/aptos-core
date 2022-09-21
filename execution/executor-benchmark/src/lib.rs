// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod account_generator;
pub mod db_generator;
pub mod pipeline;
pub mod transaction_committer;
pub mod transaction_executor;
pub mod transaction_generator;

use crate::{
    transaction_committer::TransactionCommitter, transaction_executor::TransactionExecutor,
    transaction_generator::TransactionGenerator,
};
use aptos_config::config::{
    NodeConfig, PrunerConfig, RocksdbConfigs, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    NO_OP_STORAGE_PRUNER_CONFIG, TARGET_SNAPSHOT_SIZE,
};
use aptos_jellyfish_merkle::metrics::{
    APTOS_JELLYFISH_INTERNAL_ENCODED_BYTES, APTOS_JELLYFISH_LEAF_ENCODED_BYTES,
};
use aptosdb::AptosDB;

use crate::pipeline::Pipeline;
use aptos_vm::AptosVM;
use executor::block_executor::BlockExecutor;
use std::{fs, path::Path};
use storage_interface::DbReaderWriter;

pub fn init_db_and_executor(config: &NodeConfig) -> (DbReaderWriter, BlockExecutor<AptosVM>) {
    let db = DbReaderWriter::new(
        AptosDB::open(
            &config.storage.dir(),
            false, /* readonly */
            config.storage.storage_pruner_config,
            RocksdbConfigs::default(),
            false,
            config.storage.target_snapshot_size,
            config.storage.max_num_nodes_per_lru_cache_shard,
        )
        .expect("DB should open."),
    );

    let executor = BlockExecutor::new(db.clone());

    (db, executor)
}

fn create_checkpoint(source_dir: impl AsRef<Path>, checkpoint_dir: impl AsRef<Path>) {
    // Create rocksdb checkpoint.
    if checkpoint_dir.as_ref().exists() {
        fs::remove_dir_all(checkpoint_dir.as_ref()).unwrap_or(());
    }
    std::fs::create_dir_all(checkpoint_dir.as_ref()).unwrap();

    AptosDB::open(
        &source_dir,
        false,                       /* readonly */
        NO_OP_STORAGE_PRUNER_CONFIG, /* pruner */
        RocksdbConfigs::default(),
        false,
        TARGET_SNAPSHOT_SIZE,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    )
    .expect("db open failure.")
    .create_checkpoint(checkpoint_dir.as_ref())
    .expect("db checkpoint creation fails.");
}

/// Runs the benchmark with given parameters.
pub fn run_benchmark(
    block_size: usize,
    num_transfer_blocks: usize,
    source_dir: impl AsRef<Path>,
    checkpoint_dir: impl AsRef<Path>,
    verify_sequence_numbers: bool,
    pruner_config: PrunerConfig,
) {
    create_checkpoint(source_dir.as_ref(), checkpoint_dir.as_ref());

    let (mut config, genesis_key) = aptos_genesis::test_utils::test_config();
    config.storage.dir = checkpoint_dir.as_ref().to_path_buf();
    config.storage.storage_pruner_config = pruner_config;

    let (db, executor) = init_db_and_executor(&config);
    let version = db.reader.get_latest_version().unwrap();

    let (pipeline, block_sender) = Pipeline::new(executor, version);

    let mut generator = TransactionGenerator::new_with_existing_db(
        db.clone(),
        genesis_key,
        block_sender,
        source_dir,
        version,
    );
    generator.run_transfer(block_size, num_transfer_blocks);
    generator.drop_sender();
    pipeline.join();

    if verify_sequence_numbers {
        generator.verify_sequence_numbers(db.reader);
    }
}

pub fn add_accounts(
    num_new_accounts: usize,
    init_account_balance: u64,
    block_size: usize,
    source_dir: impl AsRef<Path>,
    checkpoint_dir: impl AsRef<Path>,
    pruner_config: PrunerConfig,
    verify_sequence_numbers: bool,
) {
    assert!(source_dir.as_ref() != checkpoint_dir.as_ref());
    create_checkpoint(source_dir.as_ref(), checkpoint_dir.as_ref());
    add_accounts_impl(
        num_new_accounts,
        init_account_balance,
        block_size,
        source_dir,
        checkpoint_dir,
        pruner_config,
        verify_sequence_numbers,
    );
}

fn add_accounts_impl(
    num_new_accounts: usize,
    init_account_balance: u64,
    block_size: usize,
    source_dir: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    pruner_config: PrunerConfig,
    verify_sequence_numbers: bool,
) {
    let (mut config, genesis_key) = aptos_genesis::test_utils::test_config();
    config.storage.dir = output_dir.as_ref().to_path_buf();
    config.storage.storage_pruner_config = pruner_config;
    let (db, executor) = init_db_and_executor(&config);

    let version = db.reader.get_latest_version().unwrap();

    let (pipeline, block_sender) = Pipeline::new(executor, version);

    let mut generator = TransactionGenerator::new_with_existing_db(
        db.clone(),
        genesis_key,
        block_sender,
        &source_dir,
        version,
    );

    generator.run_mint(
        db.reader.clone(),
        generator.num_existing_accounts(),
        num_new_accounts,
        init_account_balance,
        block_size,
    );
    generator.drop_sender();
    pipeline.join();

    if verify_sequence_numbers {
        println!("Verifying sequence numbers...");
        // Do a sanity check on the sequence number to make sure all transactions are committed.
        generator.verify_sequence_numbers(db.reader);
    }

    println!(
        "Created {} new accounts. Now at version {}, total # of accounts {}.",
        num_new_accounts,
        generator.version(),
        generator.num_existing_accounts() + num_new_accounts,
    );

    // Write metadata
    generator.write_meta(&output_dir, num_new_accounts);

    println!(
        "Total written internal nodes value size: {} bytes",
        APTOS_JELLYFISH_INTERNAL_ENCODED_BYTES.get()
    );
    println!(
        "Total written leaf nodes value size: {} bytes",
        APTOS_JELLYFISH_LEAF_ENCODED_BYTES.get()
    );
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
            25, /* num_accounts */
            // TODO(Gas): double check if this is correct
            10_000, /* init_account_balance */
            5,      /* block_size */
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
