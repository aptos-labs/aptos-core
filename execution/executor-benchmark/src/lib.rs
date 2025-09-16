// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod account_generator;
pub mod block_preparation;
pub mod db_access;
pub mod db_generator;
mod db_reliable_submitter;
mod ledger_update_stage;
pub mod measurements;
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
use aptos_config::config::{NodeConfig, PrunerConfig, NO_OP_STORAGE_PRUNER_CONFIG};
use aptos_db::AptosDB;
use aptos_executor::block_executor::BlockExecutor;
use aptos_jellyfish_merkle::metrics::{
    APTOS_JELLYFISH_INTERNAL_ENCODED_BYTES, APTOS_JELLYFISH_LEAF_ENCODED_BYTES,
};
use aptos_logger::{info, warn};
use aptos_sdk::types::LocalAccount;
use aptos_storage_interface::{
    state_store::state_view::db_state_view::LatestDbStateCheckpointView, DbReader, DbReaderWriter,
};
use aptos_transaction_generator_lib::{
    create_txn_generator_creator, AlwaysApproveRootAccountHandle, TransactionGeneratorCreator,
    TransactionType::{self, CoinTransfer},
};
use aptos_types::on_chain_config::{FeatureFlag, Features};
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, AptosVM, VMBlockExecutor};
use db_generator::create_db_with_accounts;
use db_reliable_submitter::DbReliableTransactionSubmitter;
use measurements::{EventMeasurements, OverallMeasurement, OverallMeasuring};
use pipeline::PipelineConfig;
use std::{
    fs,
    path::Path,
    sync::{atomic::AtomicUsize, Arc},
    time::Instant,
};
use tokio::runtime::Runtime;

pub struct SingleRunResults {
    pub measurements: OverallMeasurement,
    pub per_stage_measurements: Vec<OverallMeasurement>,
    pub per_stage_events: EventMeasurements,
}

pub fn default_benchmark_features() -> Features {
    let mut features = Features::default();
    features.disable(FeatureFlag::CALCULATE_TRANSACTION_FEE_FOR_DISTRIBUTION);
    features
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

enum InitializedBenchmarkWorkload {
    TransactionMix {
        transaction_generators: Vec<Box<dyn aptos_transaction_generator_lib::TransactionGenerator>>,
        phase: Arc<AtomicUsize>,
        workload_name: String,
    },
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
) -> SingleRunResults
where
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

    let initialized_workload = match workload {
        BenchmarkWorkload::TransactionMix(transaction_mix) => {
            let workload_name = format!("{:?} via txn generator", transaction_mix);

            let num_existing_accounts = TransactionGenerator::read_meta(&source_dir);
            let num_accounts_to_be_loaded = std::cmp::min(
                num_existing_accounts,
                num_main_signer_accounts + num_additional_dst_pool_accounts,
            );

            let mut num_accounts_to_skip = 0;
            for (transaction_type, _) in &transaction_mix {
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
                transaction_mix,
                root_account.clone(),
                main_signer_accounts,
                burner_accounts,
                db.clone(),
                // Initialization pipeline is temporary, so needs to be fully committed.
                // No discards/aborts allowed during initialization, even if they are allowed later.
                &PipelineConfig::default(),
            );
            // need to initialize all workers and finish with all transactions before we start the timer:
            InitializedBenchmarkWorkload::TransactionMix {
                transaction_generators: (0..pipeline_config.num_generator_workers)
                    .map(|_| transaction_generator_creator.create_transaction_generator())
                    .collect::<Vec<_>>(),
                phase,
                workload_name,
            }
        },
        BenchmarkWorkload::Transfer {
            connected_tx_grps,
            shuffle_connected_txns,
            hotspot_probability,
        } => InitializedBenchmarkWorkload::Transfer {
            connected_tx_grps,
            shuffle_connected_txns,
            hotspot_probability,
        },
    };

    let start_version = db.reader.expect_synced_version();
    let executor = BlockExecutor::<V>::new(db.clone());
    let (pipeline, block_sender) =
        Pipeline::new(executor, start_version, &pipeline_config, Some(num_blocks));

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

    let (num_blocks_created, workload_name) = match initialized_workload {
        InitializedBenchmarkWorkload::TransactionMix {
            transaction_generators,
            phase,
            workload_name,
        } => {
            let num_blocks_created = generator.run_workload(
                block_size,
                num_blocks,
                transaction_generators,
                phase,
                transactions_per_sender,
            );
            (num_blocks_created, workload_name)
        },
        InitializedBenchmarkWorkload::Transfer {
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
    let (num_pipeline_txns, staged_results, staged_events) = pipeline.join();

    info!("Executed workload {}", workload_name);

    let num_txns = if !pipeline_config.skip_commit {
        db.reader.expect_synced_version() - start_version - num_blocks_created as u64
    } else {
        num_pipeline_txns.unwrap_or_default()
    };

    let overall_results =
        overall_measuring.elapsed("Overall".to_string(), "".to_string(), num_txns);
    overall_results.print_end();

    if !pipeline_config.skip_commit {
        if verify_sequence_numbers {
            generator.verify_sequence_numbers(db.reader.clone());
        }
        log_total_supply(&db.reader);
    }

    // Assert there were no error log lines in the run.
    assert_eq!(0, aptos_logger::ERROR_LOG_COUNT.get());

    OverallMeasurement::print_end_table(&staged_results, &overall_results);
    staged_events.print_end_table();
    SingleRunResults {
        measurements: overall_results,
        per_stage_measurements: staged_results,
        per_stage_events: staged_events,
    }
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
            vec![transaction_mix],
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

fn log_total_supply(db_reader: &Arc<dyn DbReader>) {
    let total_supply =
        DbAccessUtil::get_total_supply(&db_reader.latest_state_checkpoint_view().unwrap()).unwrap();
    info!("total supply is {:?} octas", total_supply)
}

pub enum SingleRunMode {
    TEST,
    BENCHMARK {
        approx_tps: usize,
        /// Number of blocks to run your test for. ~10-30 is a good number.
        /// If your workflow has an end (generats no transactions after some point),
        /// you can set a large number, and test will stop by itself.
        run_for_blocks: Option<usize>,
        additional_configs: Option<SingleRunAdditionalConfigs>,
    },
}

// Optional more detailed configuration.
pub struct SingleRunAdditionalConfigs {
    pub num_generator_workers: usize,
    pub split_stages: bool,
}

pub fn run_single_with_default_params(
    transaction_type: TransactionType,
    test_folder: impl AsRef<Path>,
    concurrency_level: usize,
    mode: SingleRunMode,
) -> SingleRunResults {
    aptos_logger::Logger::new().init();

    AptosVM::set_num_shards_once(1);
    AptosVM::set_concurrency_level_once(concurrency_level);
    AptosVM::set_processed_transactions_detailed_counters();

    rayon::ThreadPoolBuilder::new()
        .thread_name(|index| format!("rayon-global-{}", index))
        .build_global()
        .expect("Failed to build rayon global thread pool.");

    let verify_sequence_numbers = false;
    let is_keyless = false;
    let print_transactions = match mode {
        SingleRunMode::TEST => true,
        SingleRunMode::BENCHMARK { .. } => false,
    };
    let num_accounts = match mode {
        SingleRunMode::TEST => 100,
        SingleRunMode::BENCHMARK { .. } => 100000,
    };
    let num_blocks = match mode {
        SingleRunMode::TEST
        | SingleRunMode::BENCHMARK {
            run_for_blocks: None,
            ..
        } => 30,
        SingleRunMode::BENCHMARK {
            run_for_blocks: Some(num_blocks),
            ..
        } => num_blocks,
    };
    let benchmark_block_size = match mode {
        SingleRunMode::TEST => 10,
        SingleRunMode::BENCHMARK { approx_tps, .. } => {
            debug_assert!(
                false,
                "Benchmark shouldn't be run in debug mode, use --release instead."
            );
            (approx_tps / 4).clamp(10, 10000)
        },
    };
    let num_generator_workers = match mode {
        SingleRunMode::TEST
        | SingleRunMode::BENCHMARK {
            additional_configs: None,
            ..
        } => 4,
        SingleRunMode::BENCHMARK {
            additional_configs:
                Some(SingleRunAdditionalConfigs {
                    num_generator_workers,
                    ..
                }),
            ..
        } => num_generator_workers,
    };
    let split_stages = match mode {
        SingleRunMode::TEST
        | SingleRunMode::BENCHMARK {
            additional_configs: None,
            ..
        } => false,
        SingleRunMode::BENCHMARK {
            additional_configs: Some(SingleRunAdditionalConfigs { split_stages, .. }),
            ..
        } => split_stages,
    };

    let num_main_signer_accounts = num_accounts / 5;
    let num_dst_pool_accounts = num_accounts / 2;

    let storage_dir = test_folder.as_ref().join("db");
    let checkpoint_dir = test_folder.as_ref().join("cp");

    println!("db_generator::create_db_with_accounts");

    let mut features = default_benchmark_features();
    features.enable(FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE);
    features.enable(FeatureFlag::OPERATIONS_DEFAULT_TO_FA_APT_STORE);

    let init_pipeline_config = PipelineConfig {
        num_sig_verify_threads: std::cmp::max(1, num_cpus::get() / 3),
        print_transactions,
        ..Default::default()
    };

    create_db_with_accounts::<AptosVMBlockExecutor>(
        num_accounts,       /* num_accounts */
        100000 * 100000000, /* init_account_balance */
        10000,              /* block_size */
        &storage_dir,
        NO_OP_STORAGE_PRUNER_CONFIG, /* prune_window */
        verify_sequence_numbers,
        true,
        init_pipeline_config,
        features.clone(),
        is_keyless,
    );

    println!("run_benchmark");

    let execute_pipeline_config = PipelineConfig {
        generate_then_execute: true,
        num_sig_verify_threads: std::cmp::max(1, num_cpus::get() / 3),
        print_transactions,
        num_generator_workers,
        split_stages,
        ..Default::default()
    };

    run_benchmark::<AptosVMBlockExecutor>(
        benchmark_block_size, /* block_size */
        num_blocks,           /* num_blocks */
        BenchmarkWorkload::TransactionMix(vec![(transaction_type, 1)]),
        1, /* transactions per sender */
        num_main_signer_accounts,
        num_dst_pool_accounts,
        &storage_dir,
        checkpoint_dir,
        verify_sequence_numbers,
        NO_OP_STORAGE_PRUNER_CONFIG,
        true,
        execute_pipeline_config,
        features,
        is_keyless,
    )
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
    use aptos_transaction_generator_lib::WorkflowProgress;
    use aptos_transaction_workloads_lib::args::TransactionTypeArg;
    use aptos_types::{
        access_path::Path,
        account_address::AccountAddress,
        on_chain_config::{FeatureFlag, Features},
        state_store::state_key::inner::StateKeyInner,
        transaction::{
            signature_verified_transaction::into_signature_verified_block, Transaction,
            TransactionOutput, TransactionPayload,
        },
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
        let mut fa_features = default_benchmark_features();
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
                .execute_and_update_state(
                    (block_id, into_signature_verified_block(vec![txn.clone()])).into(),
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
            .execute_and_update_state(
                (block_id, into_signature_verified_block(vec![txn])).into(),
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

        assert_eq!(2, vm_to_commit.transaction_outputs.len());
        let vm_txn_output = &vm_to_commit.transaction_outputs[0];
        let vm_cp_txn_output = &vm_to_commit.transaction_outputs[1];

        assert_eq!(2, other_to_commit.transaction_outputs.len());
        let other_txn_output = &other_to_commit.transaction_outputs[0];
        let other_cp_txn_output = &other_to_commit.transaction_outputs[1];

        assert_equal_transaction_outputs(vm_cp_txn_output, other_cp_txn_output);

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
            .write_op_iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<HashMap<_, _>>();
        let other_writes = other_txn_output
            .write_set()
            .write_op_iter()
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

    // TODO(HotState): hotness computation not implemented in all VMs, so their hotness part of the
    // write set might be different.
    fn assert_equal_transaction_outputs(output1: &TransactionOutput, output2: &TransactionOutput) {
        assert_eq!(output1.write_set().as_v0(), output2.write_set().as_v0());
        assert_eq!(output1.events(), output2.events());
        assert_eq!(output1.gas_used(), output2.gas_used());
        assert_eq!(output1.status(), output2.status());
        assert_eq!(output1.auxiliary_data(), output2.auxiliary_data());
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
    fn test_benchmark_orderless_transaction() {
        AptosVM::set_num_shards_once(4);
        AptosVM::set_concurrency_level_once(4);
        AptosVM::set_processed_transactions_detailed_counters();
        NativeConfig::set_concurrency_level_once(4);
        test_generic_benchmark::<AptosVMBlockExecutor>(
            Some(TransactionTypeArg::NoOpOrderless),
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
