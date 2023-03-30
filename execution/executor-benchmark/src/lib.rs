// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod account_generator;
pub mod benchmark_transaction;
pub mod db_generator;
pub mod fake_executor;
mod metrics;
pub mod pipeline;
pub mod transaction_committer;
pub mod transaction_executor;
pub mod transaction_generator;

use crate::{
    benchmark_transaction::BenchmarkTransaction, pipeline::Pipeline,
    transaction_committer::TransactionCommitter, transaction_executor::TransactionExecutor,
    transaction_generator::TransactionGenerator,
};
use aptos_config::config::{NodeConfig, PrunerConfig};
use aptos_db::AptosDB;
use aptos_executor::block_executor::{BlockExecutor, TransactionBlockExecutor};
use aptos_jellyfish_merkle::metrics::{
    APTOS_JELLYFISH_INTERNAL_ENCODED_BYTES, APTOS_JELLYFISH_LEAF_ENCODED_BYTES,
};
use aptos_logger::info;
use aptos_storage_interface::DbReaderWriter;
use std::{fs, path::Path, time::Instant};

pub fn init_db_and_executor<V>(
    config: &NodeConfig,
) -> (DbReaderWriter, BlockExecutor<V, BenchmarkTransaction>)
where
    V: TransactionBlockExecutor<BenchmarkTransaction>,
{
    let db = DbReaderWriter::new(
        AptosDB::open(
            &config.storage.dir(),
            false, /* readonly */
            config.storage.storage_pruner_config,
            config.storage.rocksdb_configs,
            false,
            config.storage.buffered_state_target_items,
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

    AptosDB::create_checkpoint(source_dir, checkpoint_dir).expect("db checkpoint creation fails.");
}

/// Runs the benchmark with given parameters.
pub fn run_benchmark<V>(
    block_size: usize,
    num_transfer_blocks: usize,
    transactions_per_sender: usize,
    source_dir: impl AsRef<Path>,
    checkpoint_dir: impl AsRef<Path>,
    verify_sequence_numbers: bool,
    pruner_config: PrunerConfig,
    use_state_kv_db: bool,
) where
    V: TransactionBlockExecutor<BenchmarkTransaction> + 'static,
{
    create_checkpoint(source_dir.as_ref(), checkpoint_dir.as_ref());

    let (mut config, genesis_key) = aptos_genesis::test_utils::test_config();
    config.storage.dir = checkpoint_dir.as_ref().to_path_buf();
    config.storage.storage_pruner_config = pruner_config;
    config.storage.rocksdb_configs.use_state_kv_db = use_state_kv_db;

    let (db, executor) = init_db_and_executor::<V>(&config);
    let version = db.reader.get_latest_version().unwrap();

    let (pipeline, block_sender) = Pipeline::new(executor, version);

    let mut generator = TransactionGenerator::new_with_existing_db(
        db.clone(),
        genesis_key,
        block_sender,
        source_dir,
        version,
    );

    let start_time = Instant::now();
    generator.run_transfer(block_size, num_transfer_blocks, transactions_per_sender);
    generator.drop_sender();
    pipeline.join();

    let elapsed = start_time.elapsed().as_secs_f32();
    let delta_v = db.reader.get_latest_version().unwrap() - version;
    info!("Overall TPS: transfer: {} txn/s", delta_v as f32 / elapsed,);

    if verify_sequence_numbers {
        generator.verify_sequence_numbers(db.reader);
    }
}

pub fn add_accounts<V>(
    num_new_accounts: usize,
    init_account_balance: u64,
    block_size: usize,
    source_dir: impl AsRef<Path>,
    checkpoint_dir: impl AsRef<Path>,
    pruner_config: PrunerConfig,
    verify_sequence_numbers: bool,
    use_state_kv_db: bool,
) where
    V: TransactionBlockExecutor<BenchmarkTransaction> + 'static,
{
    assert!(source_dir.as_ref() != checkpoint_dir.as_ref());
    create_checkpoint(source_dir.as_ref(), checkpoint_dir.as_ref());
    add_accounts_impl::<V>(
        num_new_accounts,
        init_account_balance,
        block_size,
        source_dir,
        checkpoint_dir,
        pruner_config,
        verify_sequence_numbers,
        use_state_kv_db,
    );
}

fn add_accounts_impl<V>(
    num_new_accounts: usize,
    init_account_balance: u64,
    block_size: usize,
    source_dir: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    pruner_config: PrunerConfig,
    verify_sequence_numbers: bool,
    use_state_kv_db: bool,
) where
    V: TransactionBlockExecutor<BenchmarkTransaction> + 'static,
{
    let (mut config, genesis_key) = aptos_genesis::test_utils::test_config();
    config.storage.dir = output_dir.as_ref().to_path_buf();
    config.storage.storage_pruner_config = pruner_config;
    config.storage.rocksdb_configs.use_state_kv_db = use_state_kv_db;
    let (db, executor) = init_db_and_executor::<V>(&config);

    let version = db.reader.get_latest_version().unwrap();

    let (pipeline, block_sender) = Pipeline::new(executor, version);

    let mut generator = TransactionGenerator::new_with_existing_db(
        db.clone(),
        genesis_key,
        block_sender,
        &source_dir,
        version,
    );

    let start_time = Instant::now();
    generator.run_mint(
        db.reader.clone(),
        generator.num_existing_accounts(),
        num_new_accounts,
        init_account_balance,
        block_size,
    );
    generator.drop_sender();
    pipeline.join();

    let elapsed = start_time.elapsed().as_secs_f32();
    let delta_v = db.reader.get_latest_version().unwrap() - version;
    info!(
        "Overall TPS: account creation: {} txn/s",
        delta_v as f32 / elapsed,
    );

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
    use aptos_vm::AptosVM;

    #[test]
    fn test_benchmark() {
        let storage_dir = TempPath::new();
        let checkpoint_dir = TempPath::new();

        crate::db_generator::run::<AptosVM>(
            25, /* num_accounts */
            // TODO(Gas): double check if this is correct
            100_000_000, /* init_account_balance */
            5,           /* block_size */
            storage_dir.as_ref(),
            NO_OP_STORAGE_PRUNER_CONFIG, /* prune_window */
            true,
            false,
        );

        super::run_benchmark::<AptosVM>(
            6, /* block_size */
            5, /* num_transfer_blocks */
            2, /* transactions per sender */
            storage_dir.as_ref(),
            checkpoint_dir,
            true,
            NO_OP_STORAGE_PRUNER_CONFIG,
            false,
        );
    }
}
