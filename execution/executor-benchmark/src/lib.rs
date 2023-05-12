// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod account_generator;
pub mod benchmark_transaction;
pub mod db_access;
pub mod db_generator;
mod gen_executor;
mod metrics;
pub mod native_executor;
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
use aptos_transaction_generator_lib::{
    create_txn_generator_creator, TransactionGeneratorCreator, TransactionType,
};
use aptos_vm::counters::TXN_GAS_USAGE;
use gen_executor::DbGenInitTransactionExecutor;
use pipeline::PipelineConfig;
use std::{
    fs,
    path::Path,
    sync::{atomic::AtomicUsize, Arc},
    time::Instant,
};
use tokio::runtime::Runtime;

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

fn create_checkpoint(
    source_dir: impl AsRef<Path>,
    checkpoint_dir: impl AsRef<Path>,
    use_sharded_state_merkle_db: bool,
) {
    // Create rocksdb checkpoint.
    if checkpoint_dir.as_ref().exists() {
        fs::remove_dir_all(checkpoint_dir.as_ref()).unwrap_or(());
    }
    std::fs::create_dir_all(checkpoint_dir.as_ref()).unwrap();

    AptosDB::create_checkpoint(source_dir, checkpoint_dir, use_sharded_state_merkle_db)
        .expect("db checkpoint creation fails.");
}

/// Runs the benchmark with given parameters.
pub fn run_benchmark<V>(
    block_size: usize,
    num_blocks: usize,
    transaction_type: Option<TransactionType>,
    transactions_per_sender: usize,
    num_main_signer_accounts: usize,
    num_additional_dst_pool_accounts: usize,
    source_dir: impl AsRef<Path>,
    checkpoint_dir: impl AsRef<Path>,
    verify_sequence_numbers: bool,
    pruner_config: PrunerConfig,
    use_state_kv_db: bool,
    use_sharded_state_merkle_db: bool,
    pipeline_config: PipelineConfig,
) where
    V: TransactionBlockExecutor<BenchmarkTransaction> + 'static,
{
    create_checkpoint(
        source_dir.as_ref(),
        checkpoint_dir.as_ref(),
        use_sharded_state_merkle_db,
    );

    let (mut config, genesis_key) = aptos_genesis::test_utils::test_config();
    config.storage.dir = checkpoint_dir.as_ref().to_path_buf();
    config.storage.storage_pruner_config = pruner_config;
    config.storage.rocksdb_configs.use_state_kv_db = use_state_kv_db;
    config.storage.rocksdb_configs.use_sharded_state_merkle_db = use_sharded_state_merkle_db;

    let (db, executor) = init_db_and_executor::<V>(&config);

    let transaction_generator_creator = transaction_type.map(|transaction_type| {
        init_workload::<V, _>(
            transaction_type,
            num_main_signer_accounts,
            num_additional_dst_pool_accounts,
            db.clone(),
            &source_dir,
            // Initialization pipeline is temporary, so needs to be fully committed.
            // No discards/aborts allowed during initialization, even if they are allowed later.
            PipelineConfig {
                delay_execution_start: false,
                split_stages: false,
                skip_commit: false,
                allow_discards: false,
                allow_aborts: false,
            },
        )
    });

    let version = db.reader.get_latest_version().unwrap();

    let (pipeline, block_sender) =
        Pipeline::new(executor, version, pipeline_config.clone(), Some(num_blocks));
    let mut generator = TransactionGenerator::new_with_existing_db(
        db.clone(),
        genesis_key,
        block_sender,
        source_dir,
        version,
        Some(num_main_signer_accounts),
    );

    let mut start_time = Instant::now();
    let start_gas = TXN_GAS_USAGE.get_sample_sum();
    if let Some(transaction_generator_creator) = transaction_generator_creator {
        generator.run_workload(
            block_size,
            num_blocks,
            transaction_generator_creator,
            transactions_per_sender,
        );
    } else {
        generator.run_transfer(block_size, num_blocks, transactions_per_sender);
    }
    if pipeline_config.delay_execution_start {
        start_time = Instant::now();
    }
    pipeline.start_execution();
    generator.drop_sender();
    pipeline.join();

    let elapsed = start_time.elapsed().as_secs_f32();
    let delta_v = db.reader.get_latest_version().unwrap() - version;
    let delta_gas = TXN_GAS_USAGE.get_sample_sum() - start_gas;
    info!(
        "Executed workload {}",
        if let Some(ttype) = transaction_type {
            format!("{:?} via txn generator", ttype)
        } else {
            "raw transfer".to_string()
        }
    );
    info!("Overall TPS: {} txn/s", delta_v as f32 / elapsed);
    info!("Overall GPS: {} gas/s", delta_gas as f32 / elapsed);

    if verify_sequence_numbers {
        generator.verify_sequence_numbers(db.reader);
    }
}

fn init_workload<V, P: AsRef<Path>>(
    transaction_type: TransactionType,
    num_main_signer_accounts: usize,
    num_additional_dst_pool_accounts: usize,
    db: DbReaderWriter,
    db_dir: &P,
    pipeline_config: PipelineConfig,
) -> Box<dyn TransactionGeneratorCreator>
where
    V: TransactionBlockExecutor<BenchmarkTransaction> + 'static,
{
    let version = db.reader.get_latest_version().unwrap();
    let (pipeline, block_sender) = Pipeline::<V>::new(
        BlockExecutor::new(db.clone()),
        version,
        pipeline_config,
        None,
    );

    let runtime = Runtime::new().unwrap();

    let num_existing_accounts = TransactionGenerator::read_meta(db_dir);
    let num_cached_accounts = std::cmp::min(
        num_existing_accounts,
        num_main_signer_accounts + num_additional_dst_pool_accounts,
    );
    let accounts_cache =
        TransactionGenerator::gen_user_account_cache(db.reader.clone(), num_cached_accounts);

    let (mut main_signer_accounts, burner_accounts) =
        accounts_cache.split(num_main_signer_accounts);
    let transaction_factory = TransactionGenerator::create_transaction_factory();

    let (txn_generator_creator, _address_pool, _account_pool) = runtime.block_on(async {
        let phase = Arc::new(AtomicUsize::new(0));

        let db_gen_init_transaction_executor = DbGenInitTransactionExecutor {
            db: db.clone(),
            block_sender,
        };

        create_txn_generator_creator(
            &[vec![(transaction_type, 1)]],
            1,
            &mut main_signer_accounts,
            burner_accounts,
            &db_gen_init_transaction_executor,
            &transaction_factory,
            &transaction_factory,
            phase,
        )
        .await
    });

    pipeline.join();

    txn_generator_creator
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
    use_sharded_state_merkle_db: bool,
    pipeline_config: PipelineConfig,
) where
    V: TransactionBlockExecutor<BenchmarkTransaction> + 'static,
{
    assert!(source_dir.as_ref() != checkpoint_dir.as_ref());
    create_checkpoint(
        source_dir.as_ref(),
        checkpoint_dir.as_ref(),
        use_sharded_state_merkle_db,
    );
    add_accounts_impl::<V>(
        num_new_accounts,
        init_account_balance,
        block_size,
        source_dir,
        checkpoint_dir,
        pruner_config,
        verify_sequence_numbers,
        use_state_kv_db,
        use_sharded_state_merkle_db,
        pipeline_config,
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
    use_sharded_state_merkle_db: bool,
    pipeline_config: PipelineConfig,
) where
    V: TransactionBlockExecutor<BenchmarkTransaction> + 'static,
{
    let (mut config, genesis_key) = aptos_genesis::test_utils::test_config();
    config.storage.dir = output_dir.as_ref().to_path_buf();
    config.storage.storage_pruner_config = pruner_config;
    config.storage.rocksdb_configs.use_state_kv_db = use_state_kv_db;
    config.storage.rocksdb_configs.use_sharded_state_merkle_db = use_sharded_state_merkle_db;
    let (db, executor) = init_db_and_executor::<V>(&config);

    let version = db.reader.get_latest_version().unwrap();

    let (pipeline, block_sender) = Pipeline::new(
        executor,
        version,
        pipeline_config,
        Some(1 + num_new_accounts / block_size * 101 / 100),
    );

    let mut generator = TransactionGenerator::new_with_existing_db(
        db.clone(),
        genesis_key,
        block_sender,
        &source_dir,
        version,
        None,
    );

    let start_time = Instant::now();
    generator.run_mint(
        db.reader.clone(),
        generator.num_existing_accounts(),
        num_new_accounts,
        init_account_balance,
        block_size,
    );
    pipeline.start_execution();
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
    use crate::{
        benchmark_transaction::BenchmarkTransaction, native_executor::NativeExecutor,
        pipeline::PipelineConfig,
    };
    use aptos_config::config::NO_OP_STORAGE_PRUNER_CONFIG;
    use aptos_executor::block_executor::TransactionBlockExecutor;
    use aptos_temppath::TempPath;
    use aptos_transaction_generator_lib::args::TransactionTypeArg;
    use aptos_vm::AptosVM;

    fn test_generic_benchmark<E>(
        transaction_type: Option<TransactionTypeArg>,
        verify_sequence_numbers: bool,
    ) where
        E: TransactionBlockExecutor<BenchmarkTransaction> + 'static,
    {
        aptos_logger::Logger::new().init();

        let storage_dir = TempPath::new();
        let checkpoint_dir = TempPath::new();

        println!("db_generator::create_db_with_accounts");

        crate::db_generator::create_db_with_accounts::<E>(
            100, /* num_accounts */
            // TODO(Gas): double check if this is correct
            100_000_000, /* init_account_balance */
            5,           /* block_size */
            storage_dir.as_ref(),
            NO_OP_STORAGE_PRUNER_CONFIG, /* prune_window */
            verify_sequence_numbers,
            false,
            false,
            PipelineConfig {
                delay_execution_start: false,
                split_stages: false,
                skip_commit: false,
                allow_discards: false,
                allow_aborts: false,
            },
        );

        println!("run_benchmark");

        super::run_benchmark::<E>(
            6, /* block_size */
            5, /* num_blocks */
            transaction_type.map(|t| t.materialize(2)),
            2,  /* transactions per sender */
            25, /* num_main_signer_accounts */
            30, /* num_dst_pool_accounts */
            storage_dir.as_ref(),
            checkpoint_dir,
            verify_sequence_numbers,
            NO_OP_STORAGE_PRUNER_CONFIG,
            false,
            false,
            PipelineConfig {
                delay_execution_start: false,
                split_stages: true,
                skip_commit: false,
                allow_discards: false,
                allow_aborts: false,
            },
        );
    }

    #[test]
    fn test_benchmark() {
        test_generic_benchmark::<AptosVM>(None, true);
    }

    #[test]
    fn test_benchmark_transaction() {
        test_generic_benchmark::<AptosVM>(
            Some(TransactionTypeArg::TokenV1NFTMintAndTransferSequential),
            true,
        );
    }

    #[test]
    fn test_native_benchmark() {
        // correct execution not yet implemented, so cannot be checked for validity
        test_generic_benchmark::<NativeExecutor>(None, false);
    }
}
