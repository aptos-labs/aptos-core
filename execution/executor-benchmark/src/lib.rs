// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod account_generator;
pub mod block_preparation;
pub mod db_access;
pub mod db_generator;
mod db_reliable_submitter;
mod ledger_update_stage;
mod metrics;
pub mod native_executor;
pub mod pipeline;
pub mod transaction_committer;
pub mod transaction_executor;
pub mod transaction_generator;

use crate::{
    db_access::DbAccessUtil, pipeline::Pipeline, transaction_committer::TransactionCommitter,
    transaction_executor::TransactionExecutor, transaction_generator::TransactionGenerator,
};
use aptos_block_executor::counters::{self as block_executor_counters, GasType};
use aptos_block_partitioner::v2::counters::BLOCK_PARTITIONING_SECONDS;
use aptos_config::config::{NodeConfig, PrunerConfig};
use aptos_db::AptosDB;
use aptos_executor::{
    block_executor::{BlockExecutor, TransactionBlockExecutor},
    metrics::{
        APTOS_EXECUTOR_COMMIT_BLOCKS_SECONDS, APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS,
        APTOS_EXECUTOR_LEDGER_UPDATE_SECONDS, APTOS_EXECUTOR_OTHER_TIMERS_SECONDS,
        APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS, APTOS_PROCESSED_TXNS_OUTPUT_SIZE,
    },
};
use aptos_jellyfish_merkle::metrics::{
    APTOS_JELLYFISH_INTERNAL_ENCODED_BYTES, APTOS_JELLYFISH_LEAF_ENCODED_BYTES,
};
use aptos_logger::{info, warn};
use aptos_metrics_core::Histogram;
use aptos_sdk::types::LocalAccount;
use aptos_storage_interface::{state_view::LatestDbStateCheckpointView, DbReader, DbReaderWriter};
use aptos_transaction_generator_lib::{
    create_txn_generator_creator, TransactionGeneratorCreator, TransactionType,
    TransactionType::NonConflictingCoinTransfer,
};
use db_reliable_submitter::DbReliableTransactionSubmitter;
use pipeline::PipelineConfig;
use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{atomic::AtomicUsize, Arc},
    time::Instant,
};
use tokio::runtime::Runtime;

pub fn init_db_and_executor<V>(config: &NodeConfig) -> (DbReaderWriter, BlockExecutor<V>)
where
    V: TransactionBlockExecutor,
{
    let db = DbReaderWriter::new(
        AptosDB::open(
            config.storage.get_dir_paths(),
            false, /* readonly */
            config.storage.storage_pruner_config,
            config.storage.rocksdb_configs,
            false,
            config.storage.buffered_state_target_items,
            config.storage.max_num_nodes_per_lru_cache_shard,
            false,
        )
        .expect("DB should open."),
    );

    let executor = BlockExecutor::new(db.clone());

    (db, executor)
}

fn create_checkpoint(
    source_dir: impl AsRef<Path>,
    checkpoint_dir: impl AsRef<Path>,
    enable_storage_sharding: bool,
) {
    // Create rocksdb checkpoint.
    if checkpoint_dir.as_ref().exists() {
        fs::remove_dir_all(checkpoint_dir.as_ref()).unwrap_or(());
    }
    std::fs::create_dir_all(checkpoint_dir.as_ref()).unwrap();

    AptosDB::create_checkpoint(source_dir, checkpoint_dir, enable_storage_sharding)
        .expect("db checkpoint creation fails.");
}

/// Runs the benchmark with given parameters.
#[allow(clippy::too_many_arguments)]
pub fn run_benchmark<V>(
    block_size: usize,
    num_blocks: usize,
    transaction_mix: Option<Vec<(TransactionType, usize)>>,
    mut transactions_per_sender: usize,
    connected_tx_grps: usize,
    shuffle_connected_txns: bool,
    hotspot_probability: Option<f32>,
    num_main_signer_accounts: usize,
    num_additional_dst_pool_accounts: usize,
    source_dir: impl AsRef<Path>,
    checkpoint_dir: impl AsRef<Path>,
    verify_sequence_numbers: bool,
    pruner_config: PrunerConfig,
    enable_storage_sharding: bool,
    pipeline_config: PipelineConfig,
) where
    V: TransactionBlockExecutor + 'static,
{
    create_checkpoint(
        source_dir.as_ref(),
        checkpoint_dir.as_ref(),
        enable_storage_sharding,
    );

    let (mut config, genesis_key) = aptos_genesis::test_utils::test_config();
    config.storage.dir = checkpoint_dir.as_ref().to_path_buf();
    config.storage.storage_pruner_config = pruner_config;
    config.storage.rocksdb_configs.enable_storage_sharding = enable_storage_sharding;

    let (db, executor) = init_db_and_executor::<V>(&config);
    let transaction_generators = transaction_mix.clone().map(|transaction_mix| {
        let num_existing_accounts = TransactionGenerator::read_meta(&source_dir);
        let num_accounts_to_be_loaded = std::cmp::min(
            num_existing_accounts,
            num_main_signer_accounts + num_additional_dst_pool_accounts,
        );

        let mut num_accounts_to_skip = 0;
        for (transaction_type, _) in &transaction_mix {
            if let NonConflictingCoinTransfer{..} = transaction_type {
                // In case of random non-conflicting coin transfer using `P2PTransactionGenerator`,
                // `3*block_size` addresses is required:
                // `block_size` number of signers, and 2 groups of burn-n-recycle recipients used alternatively.
                if num_accounts_to_be_loaded < block_size * 3 {
                    panic!("Cannot guarantee random non-conflicting coin transfer using `P2PTransactionGenerator`.");
                }
                num_accounts_to_skip = block_size;
            }
        }

        let accounts_cache =
            TransactionGenerator::gen_user_account_cache(db.reader.clone(), num_accounts_to_be_loaded, num_accounts_to_skip);
        let (main_signer_accounts, burner_accounts) =
            accounts_cache.split(num_main_signer_accounts);

        let transaction_generator_creator = init_workload::<V>(
            transaction_mix,
            main_signer_accounts,
            burner_accounts,
            db.clone(),
            // Initialization pipeline is temporary, so needs to be fully committed.
            // No discards/aborts allowed during initialization, even if they are allowed later.
            &PipelineConfig::default(),
        );
        // need to initialize all workers and finish with all transactions before we start the timer:
        (0..pipeline_config.num_generator_workers).map(|_| transaction_generator_creator.create_transaction_generator()).collect::<Vec<_>>()
    });

    let version = db.reader.get_latest_version().unwrap();

    let (pipeline, block_sender) =
        Pipeline::new(executor, version, &pipeline_config, Some(num_blocks));

    let mut num_accounts_to_load = num_main_signer_accounts;
    if let Some(mix) = &transaction_mix {
        for (transaction_type, _) in mix {
            if let NonConflictingCoinTransfer { .. } = transaction_type {
                // In case of non-conflicting coin transfer,
                // `aptos_executor_benchmark::transaction_generator::TransactionGenerator` needs to hold
                // at least `block_size` number of accounts, all as signer only.
                num_accounts_to_load = block_size;
                if transactions_per_sender > 1 {
                    warn!(
                    "Overriding transactions_per_sender to 1 for non_conflicting_txns_per_block workload"
                );
                    transactions_per_sender = 1;
                }
            }
        }
    }
    let mut generator = TransactionGenerator::new_with_existing_db(
        db.clone(),
        genesis_key,
        block_sender,
        source_dir,
        Some(num_accounts_to_load),
        pipeline_config.num_generator_workers,
    );

    let mut start_time = Instant::now();
    let start_gas_measurement = GasMeasuring::start();
    let start_output_size = APTOS_PROCESSED_TXNS_OUTPUT_SIZE.get_sample_sum();
    let start_partitioning_total = BLOCK_PARTITIONING_SECONDS.get_sample_sum();
    let start_execution_total = APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS.get_sample_sum();
    let start_vm_only = APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS.get_sample_sum();
    let other_labels = vec![
        ("1.", true, "verified_state_view"),
        ("2.", true, "state_checkpoint"),
        ("2.1.", false, "sort_transactions"),
        ("2.2.", false, "calculate_for_transaction_block"),
        ("2.2.1.", false, "get_sharded_state_updates"),
        ("2.2.2.", false, "calculate_block_state_updates"),
        ("2.2.3.", false, "calculate_usage"),
        ("2.2.4.", false, "make_checkpoint"),
    ];

    let start_by_other = other_labels
        .iter()
        .map(|(_prefix, _top_level, other_label)| {
            (
                other_label.to_string(),
                APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                    .with_label_values(&[other_label])
                    .get_sample_sum(),
            )
        })
        .collect::<HashMap<_, _>>();
    let start_ledger_update_total = APTOS_EXECUTOR_LEDGER_UPDATE_SECONDS.get_sample_sum();
    let start_commit_total = APTOS_EXECUTOR_COMMIT_BLOCKS_SECONDS.get_sample_sum();

    let start_vm_time = APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS.get_sample_sum();
    if let Some(transaction_generators) = transaction_generators {
        generator.run_workload(
            block_size,
            num_blocks,
            transaction_generators,
            transactions_per_sender,
        );
    } else {
        generator.run_transfer(
            block_size,
            num_blocks,
            transactions_per_sender,
            connected_tx_grps,
            shuffle_connected_txns,
            hotspot_probability,
        );
    }
    if pipeline_config.delay_execution_start {
        start_time = Instant::now();
    }
    pipeline.start_execution();
    generator.drop_sender();
    pipeline.join();

    let elapsed = start_time.elapsed().as_secs_f64();
    let delta_v = (db.reader.get_latest_version().unwrap() - version - num_blocks as u64) as f64;
    let delta_gas = start_gas_measurement.end();
    let delta_output_size = APTOS_PROCESSED_TXNS_OUTPUT_SIZE.get_sample_sum() - start_output_size;

    let delta_vm_time = APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS.get_sample_sum() - start_vm_time;
    info!(
        "VM execution TPS {} txn/s; ({} / {})",
        (delta_v / delta_vm_time) as usize,
        delta_v,
        delta_vm_time
    );
    info!(
        "Executed workload {}",
        if let Some(mix) = transaction_mix {
            format!("{:?} via txn generator", mix)
        } else {
            "raw transfer".to_string()
        }
    );
    info!("Overall TPS: {} txn/s", delta_v / elapsed);
    info!("Overall GPS: {} gas/s", delta_gas.gas / elapsed);
    info!(
        "Overall effectiveGPS: {} gas/s",
        delta_gas.effective_block_gas / elapsed
    );
    info!("Overall ioGPS: {} gas/s", delta_gas.io_gas / elapsed);
    info!(
        "Overall executionGPS: {} gas/s",
        delta_gas.execution_gas / elapsed
    );
    info!(
        "Overall GPT: {} gas/txn",
        delta_gas.gas / (delta_gas.gas_count as f64).max(1.0)
    );
    info!(
        "Overall approx_output: {} bytes/s",
        delta_gas.approx_block_output / elapsed
    );
    info!("Overall output: {} bytes/s", delta_output_size / elapsed);

    let time_in_partitioning =
        BLOCK_PARTITIONING_SECONDS.get_sample_sum() - start_partitioning_total;

    info!(
        "Overall fraction of total: {:.3} in partitioning (component TPS: {})",
        time_in_partitioning / elapsed,
        delta_v / time_in_partitioning
    );

    let time_in_execution =
        APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS.get_sample_sum() - start_execution_total;
    info!(
        "Overall fraction of total: {:.3} in execution (component TPS: {})",
        time_in_execution / elapsed,
        delta_v / time_in_execution
    );
    let time_in_vm = APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS.get_sample_sum() - start_vm_only;
    info!(
        "Overall fraction of execution {:.3} in VM (component TPS: {})",
        time_in_vm / time_in_execution,
        delta_v / time_in_vm
    );
    for (prefix, top_level, other_label) in other_labels {
        let time_in_label = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&[other_label])
            .get_sample_sum()
            - start_by_other.get(other_label).unwrap();
        if top_level || time_in_label / time_in_execution > 0.01 {
            info!(
                "Overall fraction of execution {:.3} in {} {} (component TPS: {})",
                time_in_label / time_in_execution,
                prefix,
                other_label,
                delta_v / time_in_label
            );
        }
    }

    let time_in_ledger_update =
        APTOS_EXECUTOR_LEDGER_UPDATE_SECONDS.get_sample_sum() - start_ledger_update_total;
    info!(
        "Overall fraction of total: {:.3} in ledger update (component TPS: {})",
        time_in_ledger_update / elapsed,
        delta_v / time_in_ledger_update
    );

    let time_in_commit = APTOS_EXECUTOR_COMMIT_BLOCKS_SECONDS.get_sample_sum() - start_commit_total;
    info!(
        "Overall fraction of total: {:.4} in commit (component TPS: {})",
        time_in_commit / elapsed,
        delta_v / time_in_commit
    );

    if verify_sequence_numbers {
        generator.verify_sequence_numbers(db.reader.clone());
    }
    log_total_supply(&db.reader);
}

fn init_workload<V>(
    transaction_mix: Vec<(TransactionType, usize)>,
    mut main_signer_accounts: Vec<LocalAccount>,
    burner_accounts: Vec<LocalAccount>,
    db: DbReaderWriter,
    pipeline_config: &PipelineConfig,
) -> Box<dyn TransactionGeneratorCreator>
where
    V: TransactionBlockExecutor + 'static,
{
    let version = db.reader.get_latest_version().unwrap();
    let (pipeline, block_sender) = Pipeline::<V>::new(
        BlockExecutor::new(db.clone()),
        version,
        pipeline_config,
        None,
    );

    let runtime = Runtime::new().unwrap();
    let transaction_factory = TransactionGenerator::create_transaction_factory();

    let (txn_generator_creator, _address_pool, _account_pool) = runtime.block_on(async {
        let phase = Arc::new(AtomicUsize::new(0));

        let db_gen_init_transaction_executor = DbReliableTransactionSubmitter {
            db: db.clone(),
            block_sender,
        };

        create_txn_generator_creator(
            &[transaction_mix],
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
    enable_storage_sharding: bool,
    pipeline_config: PipelineConfig,
) where
    V: TransactionBlockExecutor + 'static,
{
    assert!(source_dir.as_ref() != checkpoint_dir.as_ref());
    create_checkpoint(
        source_dir.as_ref(),
        checkpoint_dir.as_ref(),
        enable_storage_sharding,
    );
    add_accounts_impl::<V>(
        num_new_accounts,
        init_account_balance,
        block_size,
        source_dir,
        checkpoint_dir,
        pruner_config,
        verify_sequence_numbers,
        enable_storage_sharding,
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
    enable_storage_sharding: bool,
    pipeline_config: PipelineConfig,
) where
    V: TransactionBlockExecutor + 'static,
{
    let (mut config, genesis_key) = aptos_genesis::test_utils::test_config();
    config.storage.dir = output_dir.as_ref().to_path_buf();
    config.storage.storage_pruner_config = pruner_config;
    config.storage.rocksdb_configs.enable_storage_sharding = enable_storage_sharding;
    let (db, executor) = init_db_and_executor::<V>(&config);

    let start_version = db.reader.get_latest_version().unwrap();

    let (pipeline, block_sender) = Pipeline::new(
        executor,
        start_version,
        &pipeline_config,
        Some(1 + num_new_accounts / block_size * 101 / 100),
    );

    let mut generator = TransactionGenerator::new_with_existing_db(
        db.clone(),
        genesis_key,
        block_sender,
        &source_dir,
        None,
        pipeline_config.num_generator_workers,
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
    let now_version = db.reader.get_latest_version().unwrap();
    let delta_v = now_version - start_version;
    info!(
        "Overall TPS: create_db: account creation: {} txn/s",
        delta_v as f32 / elapsed,
    );

    if verify_sequence_numbers {
        println!("Verifying sequence numbers...");
        // Do a sanity check on the sequence number to make sure all transactions are committed.
        generator.verify_sequence_numbers(db.reader.clone());
    }

    println!(
        "Created {} new accounts. Now at version {}, total # of accounts {}.",
        num_new_accounts,
        now_version,
        generator.num_existing_accounts() + num_new_accounts,
    );

    log_total_supply(&db.reader);

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

struct GasMeasurement {
    pub gas: f64,
    pub effective_block_gas: f64,

    pub io_gas: f64,
    pub execution_gas: f64,

    pub approx_block_output: f64,

    pub gas_count: u64,
}

impl GasMeasurement {
    pub fn sequential_gas_counter(gas_type: &str) -> Histogram {
        block_executor_counters::TXN_GAS
            .with_label_values(&[block_executor_counters::Mode::SEQUENTIAL, gas_type])
    }

    pub fn parallel_gas_counter(gas_type: &str) -> Histogram {
        block_executor_counters::TXN_GAS
            .with_label_values(&[block_executor_counters::Mode::PARALLEL, gas_type])
    }

    pub fn now() -> GasMeasurement {
        let gas = Self::sequential_gas_counter(GasType::NON_STORAGE_GAS).get_sample_sum()
            + Self::parallel_gas_counter(GasType::NON_STORAGE_GAS).get_sample_sum();
        let io_gas = Self::sequential_gas_counter(GasType::IO_GAS).get_sample_sum()
            + Self::parallel_gas_counter(GasType::IO_GAS).get_sample_sum();
        let execution_gas = Self::sequential_gas_counter(GasType::EXECUTION_GAS).get_sample_sum()
            + Self::parallel_gas_counter(GasType::EXECUTION_GAS).get_sample_sum();

        let gas_count = Self::sequential_gas_counter(GasType::NON_STORAGE_GAS).get_sample_count()
            + Self::parallel_gas_counter(GasType::NON_STORAGE_GAS).get_sample_count();

        let effective_block_gas = block_executor_counters::EFFECTIVE_BLOCK_GAS
            .with_label_values(&[block_executor_counters::Mode::SEQUENTIAL])
            .get_sample_sum()
            + block_executor_counters::EFFECTIVE_BLOCK_GAS
                .with_label_values(&[block_executor_counters::Mode::PARALLEL])
                .get_sample_sum();

        let approx_block_output = block_executor_counters::APPROX_BLOCK_OUTPUT_SIZE
            .with_label_values(&[block_executor_counters::Mode::SEQUENTIAL])
            .get_sample_sum()
            + block_executor_counters::APPROX_BLOCK_OUTPUT_SIZE
                .with_label_values(&[block_executor_counters::Mode::PARALLEL])
                .get_sample_sum();

        Self {
            gas,
            effective_block_gas,
            io_gas,
            execution_gas,
            approx_block_output,
            gas_count,
        }
    }
}

struct GasMeasuring {
    start: GasMeasurement,
}

impl GasMeasuring {
    pub fn start() -> Self {
        Self {
            start: GasMeasurement::now(),
        }
    }

    pub fn end(self) -> GasMeasurement {
        let end = GasMeasurement::now();

        GasMeasurement {
            gas: end.gas - self.start.gas,
            effective_block_gas: end.effective_block_gas - self.start.effective_block_gas,
            io_gas: end.io_gas - self.start.io_gas,
            execution_gas: end.execution_gas - self.start.execution_gas,
            approx_block_output: end.approx_block_output - self.start.approx_block_output,
            gas_count: end.gas_count - self.start.gas_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{native_executor::NativeExecutor, pipeline::PipelineConfig};
    use aptos_config::config::NO_OP_STORAGE_PRUNER_CONFIG;
    use aptos_executor::block_executor::TransactionBlockExecutor;
    use aptos_temppath::TempPath;
    use aptos_transaction_generator_lib::args::TransactionTypeArg;
    use aptos_vm::AptosVM;

    fn test_generic_benchmark<E>(
        transaction_type: Option<TransactionTypeArg>,
        verify_sequence_numbers: bool,
    ) where
        E: TransactionBlockExecutor + 'static,
    {
        aptos_logger::Logger::new().init();

        let storage_dir = TempPath::new();
        let checkpoint_dir = TempPath::new();

        println!("db_generator::create_db_with_accounts");

        crate::db_generator::create_db_with_accounts::<E>(
            100, /* num_accounts */
            // TODO(Gas): double check if this is correct
            100_000_000_000, /* init_account_balance */
            5,               /* block_size */
            storage_dir.as_ref(),
            NO_OP_STORAGE_PRUNER_CONFIG, /* prune_window */
            verify_sequence_numbers,
            false,
            PipelineConfig::default(),
        );

        println!("run_benchmark");

        super::run_benchmark::<E>(
            6, /* block_size */
            5, /* num_blocks */
            transaction_type.map(|t| vec![(t.materialize(2, false), 1)]),
            2,     /* transactions per sender */
            0,     /* connected txn groups in a block */
            false, /* shuffle the connected txns in a block */
            None,  /* maybe_hotspot_probability */
            25,    /* num_main_signer_accounts */
            30,    /* num_dst_pool_accounts */
            storage_dir.as_ref(),
            checkpoint_dir,
            verify_sequence_numbers,
            NO_OP_STORAGE_PRUNER_CONFIG,
            false,
            PipelineConfig::default(),
        );
    }

    #[test]
    fn test_benchmark_default() {
        test_generic_benchmark::<AptosVM>(None, true);
    }

    #[test]
    fn test_benchmark_transaction() {
        AptosVM::set_num_shards_once(1);
        AptosVM::set_concurrency_level_once(4);
        AptosVM::set_processed_transactions_detailed_counters();
        NativeExecutor::set_concurrency_level_once(4);
        test_generic_benchmark::<AptosVM>(
            Some(TransactionTypeArg::ModifyGlobalResourceAggV2),
            true,
        );
    }

    #[test]
    fn test_native_benchmark() {
        // correct execution not yet implemented, so cannot be checked for validity
        test_generic_benchmark::<NativeExecutor>(None, false);
    }
}

fn log_total_supply(db_reader: &Arc<dyn DbReader>) {
    let total_supply =
        DbAccessUtil::get_total_supply(&db_reader.latest_state_checkpoint_view().unwrap()).unwrap();
    info!("total supply is {:?} octas", total_supply)
}
