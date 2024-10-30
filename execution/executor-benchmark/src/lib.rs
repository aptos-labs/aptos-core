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
pub mod native;
pub mod pipeline;
pub mod transaction_committer;
pub mod transaction_executor;
pub mod transaction_generator;

use crate::{
    db_access::DbAccessUtil, pipeline::Pipeline, transaction_committer::TransactionCommitter,
    transaction_executor::TransactionExecutor, transaction_generator::TransactionGenerator,
};
use aptos_block_executor::counters::{
    self as block_executor_counters, GasType, BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK,
};
use aptos_config::config::{NodeConfig, PrunerConfig};
use aptos_db::AptosDB;
use aptos_executor::{
    block_executor::BlockExecutor,
    metrics::{
        COMMIT_BLOCKS, GET_BLOCK_EXECUTION_OUTPUT_BY_EXECUTING, OTHER_TIMERS,
        PROCESSED_TXNS_OUTPUT_SIZE, UPDATE_LEDGER,
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
    create_txn_generator_creator, AlwaysApproveRootAccountHandle, TransactionGeneratorCreator,
    TransactionType::{self, CoinTransfer},
};
use aptos_types::on_chain_config::{FeatureFlag, Features};
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, VMBlockExecutor};
use db_reliable_submitter::DbReliableTransactionSubmitter;
use metrics::TIMER;
use pipeline::PipelineConfig;
use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{atomic::AtomicUsize, Arc},
    time::Instant,
};
use tokio::runtime::Runtime;

pub fn default_benchmark_features() -> Features {
    let mut init_features = Features::default();
    init_features.disable(FeatureFlag::REMOVE_DETAILED_ERROR_FROM_HASH);
    init_features
}

pub fn init_db(config: &NodeConfig) -> DbReaderWriter {
    DbReaderWriter::new(
        AptosDB::open(
            config.storage.get_dir_paths(),
            false, /* readonly */
            config.storage.storage_pruner_config,
            config.storage.rocksdb_configs,
            false,
            config.storage.buffered_state_target_items,
            config.storage.max_num_nodes_per_lru_cache_shard,
            None,
        )
        .expect("DB should open."),
    )
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

pub enum BenchmarkWorkload {
    TransactionMix(Vec<(TransactionType, usize)>),
    Transfer {
        connected_tx_grps: usize,
        shuffle_connected_txns: bool,
        hotspot_probability: Option<f32>,
    },
}

/// Runs the benchmark with given parameters.
#[allow(clippy::too_many_arguments)]
pub fn run_benchmark<V>(
    block_size: usize,
    num_blocks: usize,
    workload: BenchmarkWorkload,
    mut transactions_per_sender: usize,
    num_main_signer_accounts: usize,
    num_additional_dst_pool_accounts: usize,
    source_dir: impl AsRef<Path>,
    checkpoint_dir: impl AsRef<Path>,
    verify_sequence_numbers: bool,
    pruner_config: PrunerConfig,
    enable_storage_sharding: bool,
    pipeline_config: PipelineConfig,
    init_features: Features,
    is_keyless: bool,
) where
    V: VMBlockExecutor + 'static,
{
    create_checkpoint(
        source_dir.as_ref(),
        checkpoint_dir.as_ref(),
        enable_storage_sharding,
    );
    let (mut config, genesis_key) =
        aptos_genesis::test_utils::test_config_with_custom_features(init_features);
    config.storage.dir = checkpoint_dir.as_ref().to_path_buf();
    config.storage.storage_pruner_config = pruner_config;
    config.storage.rocksdb_configs.enable_storage_sharding = enable_storage_sharding;
    let db = init_db(&config);
    let root_account = TransactionGenerator::read_root_account(genesis_key, &db);
    let root_account = Arc::new(root_account);

    let transaction_generators = if let BenchmarkWorkload::TransactionMix(transaction_mix) =
        &workload
    {
        let num_existing_accounts = TransactionGenerator::read_meta(&source_dir);
        let num_accounts_to_be_loaded = std::cmp::min(
            num_existing_accounts,
            num_main_signer_accounts + num_additional_dst_pool_accounts,
        );

        let mut num_accounts_to_skip = 0;
        for (transaction_type, _) in transaction_mix {
            if matches!(transaction_type, CoinTransfer { non_conflicting, .. } if *non_conflicting)
            {
                // In case of random non-conflicting coin transfer using `P2PTransactionGenerator`,
                // `3*block_size` addresses is required:
                // `block_size` number of signers, and 2 groups of burn-n-recycle recipients used alternatively.
                if num_accounts_to_be_loaded < block_size * 3 {
                    panic!("Cannot guarantee random non-conflicting coin transfer using `P2PTransactionGenerator`.");
                }
                num_accounts_to_skip = block_size;
            }
        }

        let accounts_cache = TransactionGenerator::gen_user_account_cache(
            db.reader.clone(),
            num_accounts_to_be_loaded,
            num_accounts_to_skip,
            is_keyless,
        );
        let (main_signer_accounts, burner_accounts) =
            accounts_cache.split(num_main_signer_accounts);

        let (transaction_generator_creator, phase) = init_workload::<AptosVMBlockExecutor>(
            transaction_mix.clone(),
            root_account.clone(),
            main_signer_accounts,
            burner_accounts,
            db.clone(),
            // Initialization pipeline is temporary, so needs to be fully committed.
            // No discards/aborts allowed during initialization, even if they are allowed later.
            &PipelineConfig::default(),
        );
        // need to initialize all workers and finish with all transactions before we start the timer:
        Some((
            (0..pipeline_config.num_generator_workers)
                .map(|_| transaction_generator_creator.create_transaction_generator())
                .collect::<Vec<_>>(),
            phase,
        ))
    } else {
        None
    };

    let start_version = db.reader.expect_synced_version();
    let executor = BlockExecutor::<V>::new(db.clone());
    let (pipeline, block_sender) =
        Pipeline::new(executor, start_version, &pipeline_config, Some(num_blocks));

    let mut num_accounts_to_load = num_main_signer_accounts;

    if let BenchmarkWorkload::TransactionMix(mix) = &workload {
        for (transaction_type, _) in mix {
            if matches!(transaction_type, CoinTransfer { non_conflicting, .. } if *non_conflicting)
            {
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
    let root_account = Arc::into_inner(root_account).unwrap();
    let mut generator = TransactionGenerator::new_with_existing_db(
        db.clone(),
        root_account,
        block_sender,
        source_dir,
        Some(num_accounts_to_load),
        pipeline_config.num_generator_workers,
        is_keyless,
    );

    let mut overall_measuring = OverallMeasuring::start();

    let (num_blocks_created, workload_name) = match workload {
        BenchmarkWorkload::TransactionMix(mix) => {
            let (transaction_generators, phase) = transaction_generators.unwrap();
            let num_blocks_created = generator.run_workload(
                block_size,
                num_blocks,
                transaction_generators,
                phase,
                transactions_per_sender,
            );
            (num_blocks_created, format!("{:?} via txn generator", mix))
        },
        BenchmarkWorkload::Transfer {
            connected_tx_grps,
            shuffle_connected_txns,
            hotspot_probability,
        } => {
            let num_blocks_created = generator.run_transfer(
                block_size,
                num_blocks,
                transactions_per_sender,
                connected_tx_grps,
                shuffle_connected_txns,
                hotspot_probability,
            );
            (num_blocks_created, "raw transfer".to_string())
        },
    };
    if pipeline_config.generate_then_execute {
        overall_measuring.start_time = Instant::now();
    }
    generator.drop_sender();
    info!("Done creating workload");
    pipeline.start_pipeline_processing();
    info!("Waiting for pipeline to finish");
    pipeline.join();

    info!("Executed workload {}", workload_name);

    if !pipeline_config.skip_commit {
        let num_txns =
            db.reader.expect_synced_version() - start_version - num_blocks_created as u64;
        overall_measuring.print_end("Overall", num_txns);

        if verify_sequence_numbers {
            generator.verify_sequence_numbers(db.reader.clone());
        }
        log_total_supply(&db.reader);
    }

    // Assert there were no error log lines in the run.
    assert_eq!(0, aptos_logger::ERROR_LOG_COUNT.get());
}

fn init_workload<V>(
    transaction_mix: Vec<(TransactionType, usize)>,
    root_account: Arc<LocalAccount>,
    mut main_signer_accounts: Vec<LocalAccount>,
    burner_accounts: Vec<LocalAccount>,
    db: DbReaderWriter,
    pipeline_config: &PipelineConfig,
) -> (Box<dyn TransactionGeneratorCreator>, Arc<AtomicUsize>)
where
    V: VMBlockExecutor + 'static,
{
    let start_version = db.reader.expect_synced_version();
    let (pipeline, block_sender) = Pipeline::<V>::new(
        BlockExecutor::new(db.clone()),
        start_version,
        pipeline_config,
        None,
    );

    let runtime = Runtime::new().unwrap();
    let transaction_factory = TransactionGenerator::create_transaction_factory();
    let phase = Arc::new(AtomicUsize::new(0));
    let phase_clone = phase.clone();
    let (txn_generator_creator, _address_pool, _account_pool) = runtime.block_on(async {
        let db_gen_init_transaction_executor = DbReliableTransactionSubmitter {
            db: db.clone(),
            block_sender,
        };

        let result = create_txn_generator_creator(
            &[transaction_mix],
            AlwaysApproveRootAccountHandle { root_account },
            &mut main_signer_accounts,
            burner_accounts,
            &db_gen_init_transaction_executor,
            &transaction_factory,
            &transaction_factory,
            phase_clone,
        )
        .await;

        drop(db_gen_init_transaction_executor);

        result
    });

    info!("Waiting for init to finish");
    pipeline.join();

    (txn_generator_creator, phase)
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
    init_features: Features,
    is_keyless: bool,
) where
    V: VMBlockExecutor + 'static,
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
        init_features,
        is_keyless,
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
    init_features: Features,
    is_keyless: bool,
) where
    V: VMBlockExecutor + 'static,
{
    let (mut config, genesis_key) =
        aptos_genesis::test_utils::test_config_with_custom_features(init_features);
    config.storage.dir = output_dir.as_ref().to_path_buf();
    config.storage.storage_pruner_config = pruner_config;
    config.storage.rocksdb_configs.enable_storage_sharding = enable_storage_sharding;
    let db = init_db(&config);
    let executor = BlockExecutor::<V>::new(db.clone());

    let start_version = db.reader.get_latest_ledger_info_version().unwrap();

    let (pipeline, block_sender) = Pipeline::new(
        executor,
        start_version,
        &pipeline_config,
        Some(1 + num_new_accounts / block_size * 101 / 100),
    );

    let mut generator = TransactionGenerator::new_with_existing_db(
        db.clone(),
        TransactionGenerator::read_root_account(genesis_key, &db),
        block_sender,
        &source_dir,
        None,
        pipeline_config.num_generator_workers,
        is_keyless,
    );

    let start_time = Instant::now();
    generator.run_mint(
        db.reader.clone(),
        generator.num_existing_accounts(),
        num_new_accounts,
        init_account_balance,
        block_size,
        is_keyless,
    );
    generator.drop_sender();
    pipeline.start_pipeline_processing();
    pipeline.join();

    let elapsed = start_time.elapsed().as_secs_f32();
    let now_version = db.reader.get_latest_ledger_info_version().unwrap();
    let delta_v = now_version - start_version;
    info!(
        "Overall TPS: create_db: account creation: {} txn/s",
        delta_v as f32 / elapsed,
    );

    if verify_sequence_numbers {
        info!("Verifying sequence numbers...");
        // Do a sanity check on the sequence number to make sure all transactions are committed.
        generator.verify_sequence_numbers(db.reader.clone());
    }

    info!(
        "Created {} new accounts. Now at version {}, total # of accounts {}.",
        num_new_accounts,
        now_version,
        generator.num_existing_accounts() + num_new_accounts,
    );

    // Assert there were no error log lines in the run.
    assert_eq!(0, aptos_logger::ERROR_LOG_COUNT.get());

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

#[derive(Debug, Clone)]
struct GasMeasurement {
    pub gas: f64,
    pub effective_block_gas: f64,

    pub io_gas: f64,
    pub execution_gas: f64,

    pub storage_fee: f64,

    pub approx_block_output: f64,

    pub gas_count: u64,

    pub speculative_abort_count: u64,
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

        let storage_fee = Self::sequential_gas_counter(GasType::STORAGE_FEE).get_sample_sum()
            + Self::parallel_gas_counter(GasType::STORAGE_FEE).get_sample_sum()
            - (Self::sequential_gas_counter(GasType::STORAGE_FEE_REFUND).get_sample_sum()
                + Self::parallel_gas_counter(GasType::STORAGE_FEE_REFUND).get_sample_sum());

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

        let speculative_abort_count = block_executor_counters::SPECULATIVE_ABORT_COUNT.get();

        Self {
            gas,
            effective_block_gas,
            io_gas,
            execution_gas,
            storage_fee,
            approx_block_output,
            gas_count,
            speculative_abort_count,
        }
    }

    pub fn elapsed_delta(self) -> Self {
        let end = Self::now();

        Self {
            gas: end.gas - self.gas,
            effective_block_gas: end.effective_block_gas - self.effective_block_gas,
            io_gas: end.io_gas - self.io_gas,
            execution_gas: end.execution_gas - self.execution_gas,
            storage_fee: end.storage_fee - self.storage_fee,
            approx_block_output: end.approx_block_output - self.approx_block_output,
            gas_count: end.gas_count - self.gas_count,
            speculative_abort_count: end.speculative_abort_count - self.speculative_abort_count,
        }
    }
}

static OTHER_LABELS: &[(&str, bool, &str)] = &[
    ("1.", true, "verified_state_view"),
    ("2.", true, "state_checkpoint"),
    ("2.1.", false, "sort_transactions"),
    ("2.2.", false, "calculate_for_transaction_block"),
    ("2.2.1.", false, "get_sharded_state_updates"),
    ("2.2.2.", false, "calculate_block_state_updates"),
    ("2.2.3.", false, "calculate_usage"),
    ("2.2.4.", false, "make_checkpoint"),
];

#[derive(Debug, Clone)]
struct ExecutionTimeMeasurement {
    output_size: f64,

    sig_verify_total_time: f64,
    partitioning_total_time: f64,
    execution_total_time: f64,
    block_executor_total_time: f64,
    block_executor_inner_total_time: f64,
    by_other: HashMap<&'static str, f64>,
    ledger_update_total: f64,
    commit_total_time: f64,
}

impl ExecutionTimeMeasurement {
    pub fn now() -> Self {
        let output_size = PROCESSED_TXNS_OUTPUT_SIZE
            .with_label_values(&["execution"])
            .get_sample_sum();

        let sig_verify_total = TIMER.with_label_values(&["sig_verify"]).get_sample_sum();
        let partitioning_total = TIMER.with_label_values(&["partition"]).get_sample_sum();
        let execution_total = TIMER.with_label_values(&["execute"]).get_sample_sum();
        let block_executor_total = GET_BLOCK_EXECUTION_OUTPUT_BY_EXECUTING.get_sample_sum();
        let block_executor_inner_total = BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK.get_sample_sum();

        let by_other = OTHER_LABELS
            .iter()
            .map(|(_prefix, _top_level, other_label)| {
                (
                    *other_label,
                    OTHER_TIMERS
                        .with_label_values(&[other_label])
                        .get_sample_sum(),
                )
            })
            .collect::<HashMap<_, _>>();
        let ledger_update_total = UPDATE_LEDGER.get_sample_sum();
        let commit_total = COMMIT_BLOCKS.get_sample_sum();

        Self {
            output_size,
            sig_verify_total_time: sig_verify_total,
            partitioning_total_time: partitioning_total,
            execution_total_time: execution_total,
            block_executor_total_time: block_executor_total,
            block_executor_inner_total_time: block_executor_inner_total,
            by_other,
            ledger_update_total,
            commit_total_time: commit_total,
        }
    }

    pub fn elapsed_delta(self) -> Self {
        let end = Self::now();

        Self {
            output_size: end.output_size - self.output_size,
            sig_verify_total_time: end.sig_verify_total_time - self.sig_verify_total_time,
            partitioning_total_time: end.partitioning_total_time - self.partitioning_total_time,
            execution_total_time: end.execution_total_time - self.execution_total_time,
            block_executor_total_time: end.block_executor_total_time
                - self.block_executor_total_time,
            block_executor_inner_total_time: end.block_executor_inner_total_time
                - self.block_executor_inner_total_time,
            by_other: end
                .by_other
                .into_iter()
                .map(|(k, v)| (k, v - self.by_other.get(&k).unwrap()))
                .collect::<HashMap<_, _>>(),
            ledger_update_total: end.ledger_update_total - self.ledger_update_total,
            commit_total_time: end.commit_total_time - self.commit_total_time,
        }
    }
}

#[derive(Debug, Clone)]
struct OverallMeasuring {
    start_time: Instant,
    start_execution: ExecutionTimeMeasurement,
    start_gas: GasMeasurement,
}

impl OverallMeasuring {
    pub fn start() -> Self {
        Self {
            start_time: Instant::now(),
            start_execution: ExecutionTimeMeasurement::now(),
            start_gas: GasMeasurement::now(),
        }
    }

    pub fn print_end(self, prefix: &str, num_txns: u64) {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let num_txns = num_txns as f64;
        let delta_execution = self.start_execution.elapsed_delta();
        let delta_gas = self.start_gas.elapsed_delta();

        info!(
            "{} TPS: {} txn/s (over {} txns, in {} s)",
            prefix,
            num_txns / elapsed,
            num_txns,
            elapsed
        );
        info!("{} GPS: {} gas/s", prefix, delta_gas.gas / elapsed);
        info!(
            "{} effectiveGPS: {} gas/s ({} effective block gas, in {} s)",
            prefix,
            delta_gas.effective_block_gas / elapsed,
            delta_gas.effective_block_gas,
            elapsed
        );
        info!(
            "{} speculative aborts: {} aborts/txn ({} aborts over {} txns)",
            prefix,
            delta_gas.speculative_abort_count as f64 / num_txns,
            delta_gas.speculative_abort_count,
            num_txns
        );
        info!("{} ioGPS: {} gas/s", prefix, delta_gas.io_gas / elapsed);
        info!(
            "{} executionGPS: {} gas/s",
            prefix,
            delta_gas.execution_gas / elapsed
        );
        info!(
            "{} GPT: {} gas/txn",
            prefix,
            delta_gas.gas / (delta_gas.gas_count as f64).max(1.0)
        );
        info!(
            "{} Storage fee: {} octas/txn",
            prefix,
            delta_gas.storage_fee / (delta_gas.gas_count as f64).max(1.0)
        );
        info!(
            "{} approx_output: {} bytes/s",
            prefix,
            delta_gas.approx_block_output / elapsed
        );
        info!(
            "{} output: {} bytes/s",
            prefix,
            delta_execution.output_size / elapsed
        );

        info!(
            "{} fraction of total: {:.4} in signature verification (component TPS: {:.1})",
            prefix,
            delta_execution.sig_verify_total_time / elapsed,
            num_txns / delta_execution.sig_verify_total_time
        );
        info!(
            "{} fraction of total: {:.4} in partitioning (component TPS: {:.1})",
            prefix,
            delta_execution.partitioning_total_time / elapsed,
            num_txns / delta_execution.partitioning_total_time
        );
        info!(
            "{} fraction of total: {:.4} in execution (component TPS: {:.1})",
            prefix,
            delta_execution.execution_total_time / elapsed,
            num_txns / delta_execution.execution_total_time
        );
        info!(
            "{} fraction of execution {:.4} in get execution output by executing (component TPS: {:.1})",
            prefix,
            delta_execution.block_executor_total_time / delta_execution.execution_total_time,
            num_txns / delta_execution.block_executor_total_time
        );
        info!(
            "{} fraction of execution {:.4} in inner block executor (component TPS: {:.1})",
            prefix,
            delta_execution.block_executor_inner_total_time / delta_execution.execution_total_time,
            num_txns / delta_execution.block_executor_inner_total_time
        );
        for (prefix, top_level, other_label) in OTHER_LABELS {
            let time_in_label = delta_execution.by_other.get(other_label).unwrap();
            if *top_level || time_in_label / delta_execution.execution_total_time > 0.01 {
                info!(
                    "{} fraction of execution {:.4} in {} {} (component TPS: {:.1})",
                    prefix,
                    time_in_label / delta_execution.execution_total_time,
                    prefix,
                    other_label,
                    num_txns / time_in_label
                );
            }
        }

        info!(
            "{} fraction of total: {:.4} in ledger update (component TPS: {:.1})",
            prefix,
            delta_execution.ledger_update_total / elapsed,
            num_txns / delta_execution.ledger_update_total
        );

        info!(
            "{} fraction of total: {:.4} in commit (component TPS: {:.1})",
            prefix,
            delta_execution.commit_total_time / elapsed,
            num_txns / delta_execution.commit_total_time
        );
    }
}

fn log_total_supply(db_reader: &Arc<dyn DbReader>) {
    let total_supply =
        DbAccessUtil::get_total_supply(&db_reader.latest_state_checkpoint_view().unwrap()).unwrap();
    info!("total supply is {:?} octas", total_supply)
}

#[cfg(test)]
mod tests {
    use crate::{
        db_generator::bootstrap_with_genesis,
        default_benchmark_features, init_db,
        native::{
            aptos_vm_uncoordinated::AptosVMParallelUncoordinatedBlockExecutor,
            native_config::NativeConfig,
            native_vm::NativeVMBlockExecutor,
            parallel_uncoordinated_block_executor::{
                NativeNoStorageRawTransactionExecutor, NativeParallelUncoordinatedBlockExecutor,
                NativeRawTransactionExecutor, NativeValueCacheRawTransactionExecutor,
            },
        },
        pipeline::PipelineConfig,
        transaction_executor::BENCHMARKS_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        transaction_generator::TransactionGenerator,
        BenchmarkWorkload,
    };
    use aptos_config::config::NO_OP_STORAGE_PRUNER_CONFIG;
    use aptos_crypto::HashValue;
    use aptos_executor::block_executor::BlockExecutor;
    use aptos_executor_types::BlockExecutorTrait;
    use aptos_sdk::{transaction_builder::aptos_stdlib, types::LocalAccount};
    use aptos_temppath::TempPath;
    use aptos_transaction_generator_lib::{args::TransactionTypeArg, WorkflowProgress};
    use aptos_types::{
        access_path::Path,
        account_address::AccountAddress,
        on_chain_config::{FeatureFlag, Features},
        state_store::state_key::inner::StateKeyInner,
        transaction::{Transaction, TransactionPayload},
    };
    use aptos_vm::{aptos_vm::AptosVMBlockExecutor, AptosVM, VMBlockExecutor};
    use itertools::Itertools;
    use move_core_types::language_storage::StructTag;
    use rand::thread_rng;
    use std::{
        collections::{BTreeMap, HashMap},
        fs,
    };

    #[test]
    fn test_compare_vm_and_vm_uncoordinated() {
        test_compare_prod_and_another_all_types::<AptosVMParallelUncoordinatedBlockExecutor>(true);
    }

    #[test]
    fn test_compare_vm_and_native() {
        test_compare_prod_and_another_all_types::<NativeVMBlockExecutor>(false);
    }

    #[test]
    fn test_compare_vm_and_native_parallel_uncoordinated() {
        test_compare_prod_and_another_all_types::<
            NativeParallelUncoordinatedBlockExecutor<NativeRawTransactionExecutor>,
        >(false);
    }

    fn test_compare_prod_and_another_all_types<E: VMBlockExecutor>(values_match: bool) {
        let mut non_fa_features = default_benchmark_features();
        non_fa_features.disable(FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE);
        non_fa_features.disable(FeatureFlag::OPERATIONS_DEFAULT_TO_FA_APT_STORE);
        // non_fa_features.disable(FeatureFlag::MODULE_EVENT_MIGRATION);
        // non_fa_features.disable(FeatureFlag::COIN_TO_FUNGIBLE_ASSET_MIGRATION);

        test_compare_prod_and_another::<E>(values_match, non_fa_features.clone(), |address| {
            aptos_stdlib::aptos_account_transfer(address, 1000)
        });

        test_compare_prod_and_another::<E>(
            values_match,
            non_fa_features,
            aptos_stdlib::aptos_account_create_account,
        );

        let mut fa_features = default_benchmark_features();
        fa_features.enable(FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE);
        fa_features.enable(FeatureFlag::OPERATIONS_DEFAULT_TO_FA_APT_STORE);
        fa_features.disable(FeatureFlag::CONCURRENT_FUNGIBLE_BALANCE);

        test_compare_prod_and_another::<E>(values_match, fa_features.clone(), |address| {
            aptos_stdlib::aptos_account_fungible_transfer_only(address, 1000)
        });

        test_compare_prod_and_another::<E>(values_match, fa_features.clone(), |address| {
            aptos_stdlib::aptos_account_transfer(address, 1000)
        });

        test_compare_prod_and_another::<E>(
            values_match,
            fa_features,
            aptos_stdlib::aptos_account_create_account,
        );
    }

    fn test_compare_prod_and_another<E: VMBlockExecutor>(
        values_match: bool,
        features: Features,
        txn_payload_f: impl Fn(AccountAddress) -> TransactionPayload,
    ) {
        aptos_logger::Logger::new().init();

        let db_dir = TempPath::new();

        fs::create_dir_all(db_dir.as_ref()).unwrap();

        bootstrap_with_genesis(&db_dir, false, features.clone());

        let (mut config, genesis_key) =
            aptos_genesis::test_utils::test_config_with_custom_features(features);
        config.storage.dir = db_dir.as_ref().to_path_buf();
        config.storage.storage_pruner_config = NO_OP_STORAGE_PRUNER_CONFIG;
        config.storage.rocksdb_configs.enable_storage_sharding = false;

        let (txn, vm_result) = {
            let vm_db = init_db(&config);
            let vm_executor = BlockExecutor::<AptosVMBlockExecutor>::new(vm_db.clone());

            let root_account = TransactionGenerator::read_root_account(genesis_key, &vm_db);
            let dst = LocalAccount::generate(&mut thread_rng());

            let txn_factory = TransactionGenerator::create_transaction_factory();
            let txn =
                Transaction::UserTransaction(root_account.sign_with_transaction_builder(
                    txn_factory.payload(txn_payload_f(dst.address())),
                ));
            let parent_block_id = vm_executor.committed_block_id();
            let block_id = HashValue::random();
            vm_executor
                .execute_and_state_checkpoint(
                    (block_id, vec![txn.clone()]).into(),
                    parent_block_id,
                    BENCHMARKS_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
                )
                .unwrap();

            let result = vm_executor
                .ledger_update(block_id, parent_block_id)
                .unwrap()
                .execution_output;
            result.check_aborts_discards_retries(false, false, false);
            (txn, result)
        };

        let other_db = init_db(&config);
        let other_executor = BlockExecutor::<E>::new(other_db.clone());

        let parent_block_id = other_executor.committed_block_id();
        let block_id = HashValue::random();
        other_executor
            .execute_and_state_checkpoint(
                (block_id, vec![txn]).into(),
                parent_block_id,
                BENCHMARKS_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
            )
            .unwrap();
        let other_result = other_executor
            .ledger_update(block_id, parent_block_id)
            .unwrap()
            .execution_output;
        other_result.check_aborts_discards_retries(false, false, false);

        let vm_to_commit = &vm_result.to_commit;
        let other_to_commit = &other_result.to_commit;

        assert_eq!(2, vm_to_commit.transaction_outputs().len());
        let vm_txn_output = &vm_to_commit.transaction_outputs()[0];
        let vm_cp_txn_output = &vm_to_commit.transaction_outputs()[1];

        assert_eq!(2, other_to_commit.transaction_outputs().len());
        let other_txn_output = &other_to_commit.transaction_outputs()[0];
        let other_cp_txn_output = &other_to_commit.transaction_outputs()[1];

        assert_eq!(vm_cp_txn_output, other_cp_txn_output);

        let vm_event_types = vm_txn_output
            .events()
            .iter()
            .map(|event| event.type_tag().clone())
            .sorted()
            .collect::<Vec<_>>();
        let other_event_types = other_txn_output
            .events()
            .iter()
            .map(|event| event.type_tag().clone())
            .sorted()
            .collect::<Vec<_>>();
        assert_eq!(vm_event_types, other_event_types);

        if values_match {
            for (event1, event2) in vm_txn_output
                .events()
                .iter()
                .zip_eq(other_txn_output.events().iter())
            {
                assert_eq!(event1, event2);
            }
        }

        let vm_writes = vm_txn_output
            .write_set()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<HashMap<_, _>>();
        let other_writes = other_txn_output
            .write_set()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<HashMap<_, _>>();
        for (key, value) in vm_writes.iter() {
            if let StateKeyInner::AccessPath(apath) = key.inner() {
                if let Path::ResourceGroup(_) = apath.get_path() {
                    let vm_resources =
                        bcs::from_bytes::<BTreeMap<StructTag, Vec<u8>>>(value.bytes().unwrap())
                            .unwrap();
                    let other_resources =
                        other_writes
                            .get(key)
                            .map_or_else(BTreeMap::new, |other_value| {
                                bcs::from_bytes::<BTreeMap<StructTag, Vec<u8>>>(
                                    other_value.bytes().unwrap(),
                                )
                                .unwrap()
                            });

                    assert_eq!(
                        vm_resources.keys().collect::<Vec<_>>(),
                        other_resources.keys().collect::<Vec<_>>()
                    );
                    if values_match {
                        assert_eq!(vm_resources, other_resources);
                    }
                }
            }

            assert!(other_writes.contains_key(key), "missing: {:?}", key);
            if values_match {
                let other_value = other_writes.get(key).unwrap();
                assert_eq!(value, other_value, "different value for key: {:?}", key);
            }
        }
        assert_eq!(vm_writes.len(), other_writes.len());

        if values_match {
            assert_eq!(vm_txn_output, other_txn_output);
        }
    }

    fn test_generic_benchmark<E>(
        transaction_type: Option<TransactionTypeArg>,
        verify_sequence_numbers: bool,
    ) where
        E: VMBlockExecutor + 'static,
    {
        aptos_logger::Logger::new().init();

        let storage_dir = TempPath::new();
        let checkpoint_dir = TempPath::new();

        println!("db_generator::create_db_with_accounts");

        let mut features = default_benchmark_features();
        features.enable(FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE);
        features.enable(FeatureFlag::OPERATIONS_DEFAULT_TO_FA_APT_STORE);

        crate::db_generator::create_db_with_accounts::<AptosVMBlockExecutor>(
            100, /* num_accounts */
            // TODO(Gas): double check if this is correct
            100_000_000_000, /* init_account_balance */
            5,               /* block_size */
            storage_dir.as_ref(),
            NO_OP_STORAGE_PRUNER_CONFIG, /* prune_window */
            verify_sequence_numbers,
            false,
            PipelineConfig::default(),
            features.clone(),
            false,
        );

        println!("run_benchmark");

        super::run_benchmark::<E>(
            10, /* block_size */
            30, /* num_blocks */
            transaction_type.map_or_else(
                || BenchmarkWorkload::Transfer {
                    connected_tx_grps: 0,
                    shuffle_connected_txns: false,
                    hotspot_probability: None,
                },
                |t| {
                    BenchmarkWorkload::TransactionMix(vec![(
                        t.materialize(1, true, WorkflowProgress::MoveByPhases),
                        1,
                    )])
                },
            ),
            2,  /* transactions per sender */
            25, /* num_main_signer_accounts */
            30, /* num_dst_pool_accounts */
            storage_dir.as_ref(),
            checkpoint_dir,
            verify_sequence_numbers,
            NO_OP_STORAGE_PRUNER_CONFIG,
            false,
            PipelineConfig::default(),
            features,
            false,
        );
    }

    #[test]
    fn test_benchmark_default() {
        test_generic_benchmark::<AptosVMBlockExecutor>(None, true);
    }

    #[test]
    fn test_publish_transaction() {
        AptosVM::set_num_shards_once(1);
        AptosVM::set_concurrency_level_once(4);
        AptosVM::set_processed_transactions_detailed_counters();
        test_generic_benchmark::<AptosVMBlockExecutor>(
            Some(TransactionTypeArg::RepublishAndCall),
            true,
        );
    }

    #[test]
    fn test_benchmark_transaction() {
        AptosVM::set_num_shards_once(4);
        AptosVM::set_concurrency_level_once(4);
        AptosVM::set_processed_transactions_detailed_counters();
        NativeConfig::set_concurrency_level_once(4);
        test_generic_benchmark::<AptosVMBlockExecutor>(
            Some(TransactionTypeArg::ModifyGlobalMilestoneAggV2),
            true,
        );
    }

    #[test]
    fn test_native_vm_benchmark_transaction() {
        test_generic_benchmark::<NativeVMBlockExecutor>(
            Some(TransactionTypeArg::AptFaTransfer),
            true,
        );
    }

    #[test]
    fn test_native_loose_block_executor_benchmark() {
        // correct execution not yet implemented, so cannot be checked for validity
        test_generic_benchmark::<
            NativeParallelUncoordinatedBlockExecutor<NativeRawTransactionExecutor>,
        >(Some(TransactionTypeArg::NoOp), false);
    }

    #[test]
    fn test_native_value_cache_loose_block_executor_benchmark() {
        // correct execution not yet implemented, so cannot be checked for validity
        test_generic_benchmark::<
            NativeParallelUncoordinatedBlockExecutor<NativeValueCacheRawTransactionExecutor>,
        >(Some(TransactionTypeArg::NoOp), false);
    }

    #[test]
    fn test_native_direct_raw_loose_block_executor_benchmark() {
        // correct execution not yet implemented, so cannot be checked for validity
        test_generic_benchmark::<
            NativeParallelUncoordinatedBlockExecutor<NativeNoStorageRawTransactionExecutor>,
        >(Some(TransactionTypeArg::NoOp), false);
    }
}
