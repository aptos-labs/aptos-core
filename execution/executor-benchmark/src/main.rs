// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_block_partitioner::{
    pre_partition::{
        connected_component::config::ConnectedComponentPartitionerConfig,
        default_pre_partitioner_config, uniform_partitioner::config::UniformPartitionerConfig,
        PrePartitionerConfig,
    },
    v2::config::PartitionerV2Config,
};
use aptos_config::config::{
    EpochSnapshotPrunerConfig, LedgerPrunerConfig, PrunerConfig, StateMerklePrunerConfig,
};
use aptos_executor_benchmark::{
    default_benchmark_features,
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
    BenchmarkWorkload,
};
use aptos_executor_service::remote_executor_client;
use aptos_experimental_ptx_executor::PtxBlockExecutor;
#[cfg(target_os = "linux")]
use aptos_experimental_runtimes::thread_manager::{ThreadConfigStrategy, ThreadManagerBuilder};
use aptos_metrics_core::{register_int_gauge, IntGauge};
use aptos_profiler::{ProfilerConfig, ProfilerHandler};
use aptos_push_metrics::MetricsPusher;
use aptos_transaction_generator_lib::WorkflowProgress;
use aptos_transaction_workloads_lib::args::TransactionTypeArg;
use aptos_types::on_chain_config::{FeatureFlag, Features};
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, AptosVM, VMBlockExecutor};
use aptos_vm_environment::prod_configs::set_paranoid_type_checks;
use clap::{Parser, Subcommand, ValueEnum};
use once_cell::sync::Lazy;
use std::{
    net::SocketAddr,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

/// This is needed for filters on the Grafana dashboard working as its used to populate the filter
/// variables.
pub static START_TIME: Lazy<IntGauge> =
    Lazy::new(|| register_int_gauge!("node_process_start_time", "Start time").unwrap());

#[derive(Debug, Parser)]
struct PrunerOpt {
    #[clap(long)]
    enable_state_pruner: bool,

    #[clap(long)]
    enable_epoch_snapshot_pruner: bool,

    #[clap(long)]
    enable_ledger_pruner: bool,

    #[clap(long, default_value_t = 100000)]
    state_prune_window: u64,

    #[clap(long, default_value_t = 100000)]
    epoch_snapshot_prune_window: u64,

    #[clap(long, default_value_t = 100000)]
    ledger_prune_window: u64,

    #[clap(long, default_value_t = 500)]
    ledger_pruning_batch_size: usize,

    #[clap(long, default_value_t = 500)]
    state_pruning_batch_size: usize,

    #[clap(long, default_value_t = 500)]
    epoch_snapshot_pruning_batch_size: usize,
}

impl PrunerOpt {
    fn pruner_config(&self) -> PrunerConfig {
        PrunerConfig {
            state_merkle_pruner_config: StateMerklePrunerConfig {
                enable: self.enable_state_pruner,
                prune_window: self.state_prune_window,
                batch_size: self.state_pruning_batch_size,
            },
            epoch_snapshot_pruner_config: EpochSnapshotPrunerConfig {
                enable: self.enable_epoch_snapshot_pruner,
                prune_window: self.epoch_snapshot_prune_window,
                batch_size: self.epoch_snapshot_pruning_batch_size,
            },
            ledger_pruner_config: LedgerPrunerConfig {
                enable: self.enable_ledger_pruner,
                prune_window: self.ledger_prune_window,
                batch_size: self.ledger_pruning_batch_size,
                user_pruning_window_offset: 0,
            },
        }
    }
}

#[derive(Debug, Parser)]
pub struct PipelineOpt {
    /// First generate all transactions for all blocks (and keep them in memory),
    /// and only then start the pipeline.
    /// Useful when not running large number of blocks (so it can fit in memory),
    /// as generation of blocks takes not-insignificant amount of CPU.
    #[clap(long)]
    generate_then_execute: bool,
    /// Run each stage separately, i.e. each stage wait for previous stage to finish
    /// processing all blocks, before starting.
    /// Allows to see individual throughput of each stage, avoiding resource contention.
    #[clap(long)]
    split_stages: bool,
    /// Skip commit stage - i.e. create executed blocks in memory, but never commit them.
    /// Useful when commit is the bottleneck, to see throughput of the rest of the pipeline.
    #[clap(long)]
    skip_commit: bool,
    /// Whether transactions are allowed to abort.
    /// By default, workload generates transactions that are all expected to succeeded,
    /// so aborts are not allowed - to catch any correctness/configuration issues.
    #[clap(long)]
    allow_aborts: bool,
    /// Whether transactions are allowed to be discarded.
    /// By default, workload generates transactions that are all expected to succeeded,
    /// so discards are not allowed - to catch any correctness/configuration issues.
    #[clap(long)]
    allow_discards: bool,
    /// Whether transactions are allowed to be retried.
    /// By default, workload generates transactions that are all expected to succeeded,
    /// so retries are not allowed - to catch any correctness/configuration issues.
    #[clap(long)]
    allow_retries: bool,
    /// Number of worker threads transaction generation will use.
    #[clap(long, default_value = "4")]
    num_generator_workers: usize,
    /// Number of worker threads signature verification will use.
    #[clap(long, default_value = "8")]
    num_sig_verify_threads: usize,
    /// Sharding configuration.
    #[clap(flatten)]
    sharding_opt: ShardingOpt,
    /// Set this flag to run (execute) scheduled transactions after they are scheduled.
    #[clap(long)]
    run_scheduled_txns: bool,
}

impl PipelineOpt {
    fn pipeline_config(&self) -> PipelineConfig {
        PipelineConfig {
            generate_then_execute: self.generate_then_execute,
            split_stages: self.split_stages,
            skip_commit: self.skip_commit,
            allow_aborts: self.allow_aborts,
            allow_discards: self.allow_discards,
            allow_retries: self.allow_retries,
            num_executor_shards: self.sharding_opt.num_executor_shards,
            num_generator_workers: self.num_generator_workers,
            partitioner_config: self.sharding_opt.partitioner_config(),
            num_sig_verify_threads: self.num_sig_verify_threads,
            run_scheduled_txns: self.run_scheduled_txns,
            print_transactions: false,
        }
    }
}

#[derive(Debug, Parser)]
struct ShardingOpt {
    #[clap(long, default_value = "0")]
    num_executor_shards: usize,
    #[clap(long)]
    use_global_executor: bool,
    /// Gives an option to specify remote shard addresses. If specified, then we expect the number
    /// of remote addresses to be equal to 'num_executor_shards', and one coordinator address
    /// Address is specified as <IP>:<PORT>
    #[clap(long, num_args = 1..)]
    remote_executor_addresses: Option<Vec<SocketAddr>>,
    #[clap(long)]
    coordinator_address: Option<SocketAddr>,
    #[clap(long, default_value = "4")]
    max_partitioning_rounds: usize,
    #[clap(long, default_value = "0.90")]
    partitioner_cross_shard_dep_avoid_threshold: f32,
    #[clap(long)]
    partitioner_version: Option<String>,
    #[clap(long)]
    pre_partitioner: Option<String>,
    #[clap(long, default_value = "2.0")]
    load_imbalance_tolerance: f32,
    #[clap(long, default_value = "8")]
    partitioner_v2_num_threads: usize,
    #[clap(long, default_value = "64")]
    partitioner_v2_dashmap_num_shards: usize,
}

impl ShardingOpt {
    fn pre_partitioner_config(&self) -> Box<dyn PrePartitionerConfig> {
        match self.pre_partitioner.as_deref() {
            None => default_pre_partitioner_config(),
            Some("uniform") => Box::new(UniformPartitionerConfig {}),
            Some("connected-component") => Box::new(ConnectedComponentPartitionerConfig {
                load_imbalance_tolerance: self.load_imbalance_tolerance,
            }),
            _ => panic!("Unknown PrePartitioner: {:?}", self.pre_partitioner),
        }
    }

    fn partitioner_config(&self) -> PartitionerV2Config {
        match self.partitioner_version.as_deref() {
            Some("v2") => PartitionerV2Config {
                num_threads: self.partitioner_v2_num_threads,
                max_partitioning_rounds: self.max_partitioning_rounds,
                cross_shard_dep_avoid_threshold: self.partitioner_cross_shard_dep_avoid_threshold,
                dashmap_num_shards: self.partitioner_v2_dashmap_num_shards,
                partition_last_round: !self.use_global_executor,
                pre_partitioner_config: self.pre_partitioner_config(),
            },
            None => PartitionerV2Config::default(),
            _ => panic!(
                "Unknown partitioner version: {:?}",
                self.partitioner_version
            ),
        }
    }
}

#[derive(Parser, Debug)]
struct ProfilerOpt {
    #[clap(long)]
    cpu_profiling: bool,

    #[clap(long)]
    memory_profiling: bool,
}

#[derive(Parser, Debug, ValueEnum, Clone, Default)]
enum BlockExecutorTypeOpt {
    /// Transaction execution: AptosVM
    /// Executing conflicts: in the input order, via BlockSTM,
    /// State: BlockSTM-provided MVHashMap-based view with caching
    #[default]
    AptosVMWithBlockSTM,
    /// Transaction execution: NativeVM - a simplified rust implemtation to create VMChangeSet,
    /// Executing conflicts: in the input order, via BlockSTM
    /// State: BlockSTM-provided MVHashMap-based view with caching
    NativeVMWithBlockSTM,
    /// Transaction execution: AptosVM
    /// Executing conflicts: All transactions execute on the state at the beginning of the block
    /// State: Raw CachedStateView
    AptosVMParallelUncoordinated,
    /// Transaction execution: Native rust code producing WriteSet
    /// Executing conflicts: All transactions execute on the state at the beginning of the block
    /// State: Raw CachedStateView
    NativeParallelUncoordinated,
    /// Transaction execution: Native rust code updating in-memory state, no WriteSet output
    /// Executing conflicts: All transactions execute on the state in the first come - first serve basis
    /// State: In-memory DashMap with rust values of state (i.e. StateKey -> Resource (either Account or FungibleStore)),
    ///        cached across blocks, filled upon first request
    NativeValueCacheParallelUncoordinated,
    /// Transaction execution: Native rust code updating in-memory state, no WriteSet output
    /// Executing conflicts: All transactions execute on the state in the first come - first serve basis
    /// State: In-memory DashMap with AccountAddress to seq_num and balance (ignoring all other fields).
    ///        kept across blocks, randomly initialized on first access, storage ignored.
    NativeNoStorageParallelUncoordinated,
    PtxExecutor,
}

#[derive(Parser, Debug)]
struct Opt {
    #[clap(long)]
    use_keyless_accounts: bool,

    #[clap(long, default_value_t = 10000)]
    block_size: usize,

    #[clap(long, default_value_t = 5)]
    transactions_per_sender: usize,

    /// 0 implies random TX generation; if non-zero, then 'transactions_per_sender is ignored
    /// 'connected_tx_grps' should be less than 'block_size'
    #[clap(long, default_value_t = 0)]
    connected_tx_grps: usize,

    #[clap(long)]
    shuffle_connected_txns: bool,

    #[clap(long, conflicts_with_all = &["connected_tx_grps", "transactions_per_sender"])]
    hotspot_probability: Option<f32>,

    #[clap(
        long,
        help = "Number of threads to use for execution. Generally replaces --concurrency-level flag (directly for default case, and as a total across all shards for sharded case)"
    )]
    execution_threads: Option<usize>,

    #[clap(flatten)]
    pruner_opt: PrunerOpt,

    #[clap(long)]
    enable_storage_sharding: bool,

    #[clap(flatten)]
    pipeline_opt: PipelineOpt,

    #[clap(subcommand)]
    cmd: Command,

    /// Verify sequence number of all the accounts after execution finishes
    #[clap(long)]
    verify_sequence_numbers: bool,

    #[clap(long, value_enum, ignore_case = true)]
    block_executor_type: BlockExecutorTypeOpt,

    #[clap(flatten)]
    profiler_opt: ProfilerOpt,

    #[clap(long)]
    skip_paranoid_checks: bool,
}

impl Opt {
    fn execution_threads(&self) -> usize {
        match self.execution_threads {
            None => {
                let cores = num_cpus::get();
                println!("\nExecution threads defaults to number of cores: {}", cores,);
                cores
            },
            Some(threads) => threads,
        }
    }
}

#[derive(Subcommand, Debug)]
enum Command {
    CreateDb {
        #[clap(long, value_parser)]
        data_dir: PathBuf,

        #[clap(long, default_value_t = 1000000)]
        num_accounts: usize,

        #[clap(long, default_value_t = 10000000000)]
        init_account_balance: u64,

        #[clap(
            long,
            num_args=1..,
            value_delimiter = ' ',
            help = "Optional custom enabling/disabling of the feature flags in the Move source. Enable / disable flags cannot overlap.\
            Sample usage: --enable-feature=V1 --disable-feature=V2 V3 where V1, V2, V3 are FeatureFlag enum variants.")]
        enable_feature: Vec<FeatureFlag>,

        #[clap(
            long,
            num_args=1..,
            value_delimiter = ' ',
            help = "Optional custom enabling/disabling of the feature flags in the Move source. Enable / disable flags cannot overlap.\
            Sample usage: --enable-feature=V1 --disable-feature=V2 V3 where V1, V2, V3 are FeatureFlag enum variants.")]
        disable_feature: Vec<FeatureFlag>,
    },
    RunExecutor {
        /// number of transfer blocks to run
        #[clap(long, default_value_t = 1000)]
        blocks: usize,

        #[clap(long, default_value_t = 1000000)]
        main_signer_accounts: usize,

        #[clap(long, default_value_t = 0)]
        additional_dst_pool_accounts: usize,

        /// Workload (transaction type). Uses raw coin transfer if not set,
        /// and if set uses transaction-generator-lib to generate it
        #[clap(
            long,
            value_enum,
            num_args = 0..,
            ignore_case = true
        )]
        transaction_type: Vec<TransactionTypeArg>,

        #[clap(long, num_args = 0..)]
        transaction_weights: Vec<usize>,

        #[clap(long, default_value_t = 1)]
        module_working_set_size: usize,

        #[clap(long)]
        use_sender_account_pool: bool,

        #[clap(long, value_parser)]
        data_dir: PathBuf,

        #[clap(long, value_parser)]
        checkpoint_dir: PathBuf,

        #[clap(
            long,
            num_args=1..,
            value_delimiter = ' ',
            help = "Optional custom enabling/disabling of the feature flags in the Move source. Enable / disable flags cannot overlap.\
            Sample usage: --enable-feature=V1 --disable-feature=V2 V3 where V1, V2, V3 are FeatureFlag enum variants.")]
        enable_feature: Vec<FeatureFlag>,

        #[clap(
            long,
            num_args=1..,
            value_delimiter = ' ',
            help = "Optional custom enabling/disabling of the feature flags in the Move source. Enable / disable flags cannot overlap.\
            Sample usage: --enable-feature=V1 --disable-feature=V2 V3 where V1, V2, V3 are FeatureFlag enum variants.")]
        disable_feature: Vec<FeatureFlag>,
    },
    AddAccounts {
        #[clap(long, value_parser)]
        data_dir: PathBuf,

        #[clap(long, value_parser)]
        checkpoint_dir: PathBuf,

        #[clap(long, default_value_t = 1000000)]
        num_new_accounts: usize,

        #[clap(long, default_value_t = 1000000)]
        init_account_balance: u64,
    },
}

fn get_init_features(
    enable_feature: Vec<FeatureFlag>,
    disable_feature: Vec<FeatureFlag>,
) -> Features {
    // this check is O(|enable_feature| * |disable_feature|)
    assert!(
        enable_feature.iter().all(|f| !disable_feature.contains(f)),
        "Enable and disable feature flags cannot overlap."
    );

    let mut init_features = default_benchmark_features();
    for feature in enable_feature.iter() {
        init_features.enable(*feature);
    }
    for feature in disable_feature.iter() {
        init_features.disable(*feature);
    }
    init_features
}

fn run<E>(opt: Opt)
where
    E: VMBlockExecutor + 'static,
{
    match opt.cmd {
        Command::CreateDb {
            data_dir,
            num_accounts,
            init_account_balance,
            enable_feature,
            disable_feature,
        } => {
            aptos_executor_benchmark::db_generator::create_db_with_accounts::<E>(
                num_accounts,
                init_account_balance,
                opt.block_size,
                data_dir,
                opt.pruner_opt.pruner_config(),
                opt.verify_sequence_numbers,
                opt.enable_storage_sharding,
                opt.pipeline_opt.pipeline_config(),
                get_init_features(enable_feature, disable_feature),
                opt.use_keyless_accounts,
            );
        },
        Command::RunExecutor {
            blocks,
            main_signer_accounts,
            additional_dst_pool_accounts,
            transaction_type,
            transaction_weights,
            module_working_set_size,
            use_sender_account_pool,
            data_dir,
            checkpoint_dir,
            enable_feature,
            disable_feature,
        } => {
            // aptos_types::on_chain_config::hack_enable_default_features_for_genesis(enable_feature);
            // aptos_types::on_chain_config::hack_disable_default_features_for_genesis(
            //     disable_feature,
            // );

            let workload = if transaction_type.is_empty() {
                BenchmarkWorkload::Transfer {
                    connected_tx_grps: opt.connected_tx_grps,
                    shuffle_connected_txns: opt.shuffle_connected_txns,
                    hotspot_probability: opt.hotspot_probability,
                }
            } else {
                let mix_per_phase = TransactionTypeArg::args_to_transaction_mix_per_phase(
                    &transaction_type,
                    &transaction_weights,
                    &[],
                    module_working_set_size,
                    use_sender_account_pool,
                    WorkflowProgress::MoveByPhases,
                );
                assert!(mix_per_phase.len() == 1);
                BenchmarkWorkload::TransactionMix(mix_per_phase[0].clone())
            };

            if let Some(hotspot_probability) = opt.hotspot_probability {
                if !(0.5..1.0).contains(&hotspot_probability) {
                    panic!(
                        "Parameter hotspot-probability has to be a decimal number in [0.5, 1.0)."
                    );
                }
            }

            aptos_executor_benchmark::run_benchmark::<E>(
                opt.block_size,
                blocks,
                workload,
                opt.transactions_per_sender,
                main_signer_accounts,
                additional_dst_pool_accounts,
                data_dir,
                checkpoint_dir,
                opt.verify_sequence_numbers,
                opt.pruner_opt.pruner_config(),
                opt.enable_storage_sharding,
                opt.pipeline_opt.pipeline_config(),
                get_init_features(enable_feature, disable_feature),
                opt.use_keyless_accounts,
            );
        },
        Command::AddAccounts {
            data_dir,
            checkpoint_dir,
            num_new_accounts,
            init_account_balance,
        } => {
            aptos_executor_benchmark::add_accounts::<E>(
                num_new_accounts,
                init_account_balance,
                opt.block_size,
                data_dir,
                checkpoint_dir,
                opt.pruner_opt.pruner_config(),
                opt.verify_sequence_numbers,
                opt.enable_storage_sharding,
                opt.pipeline_opt.pipeline_config(),
                Features::default(),
                opt.use_keyless_accounts,
            );
        },
    }
}

fn main() {
    let opt = Opt::parse();
    aptos_logger::Logger::new().init();
    START_TIME.set(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
    );
    rayon::ThreadPoolBuilder::new()
        .thread_name(|index| format!("rayon-global-{}", index))
        .build_global()
        .expect("Failed to build rayon global thread pool.");

    aptos_node_resource_metrics::register_node_metrics_collector(None);
    let _mp = MetricsPusher::start_for_local_run("executor-benchmark");

    let execution_threads = opt.execution_threads();
    let execution_shards = opt.pipeline_opt.sharding_opt.num_executor_shards;
    let mut execution_threads_per_shard = execution_threads;
    if execution_shards > 1 {
        assert!(
            execution_threads % execution_shards == 0,
            "Execution threads ({}) must be divisible by the number of execution shards ({}).",
            execution_threads,
            execution_shards
        );
        execution_threads_per_shard = execution_threads / execution_shards;
    }

    if opt
        .pipeline_opt
        .sharding_opt
        .remote_executor_addresses
        .is_some()
    {
        remote_executor_client::set_remote_addresses(
            opt.pipeline_opt
                .sharding_opt
                .remote_executor_addresses
                .clone()
                .unwrap(),
        );
        assert_eq!(
            execution_shards,
            remote_executor_client::get_remote_addresses().len(),
            "Number of execution shards ({}) must be equal to the number of remote addresses ({}).",
            execution_shards,
            remote_executor_client::get_remote_addresses().len()
        );
        remote_executor_client::set_coordinator_address(
            opt.pipeline_opt.sharding_opt.coordinator_address.unwrap(),
        );
        // it does not matter because shards are on remote node, but for sake of correctness lets
        // set it
        execution_threads_per_shard = execution_threads;
    }

    if opt.skip_paranoid_checks {
        set_paranoid_type_checks(false);
    }
    AptosVM::set_num_shards_once(execution_shards);
    AptosVM::set_concurrency_level_once(execution_threads_per_shard);
    NativeConfig::set_concurrency_level_once(execution_threads_per_shard);
    AptosVM::set_processed_transactions_detailed_counters();

    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);

    let cpu_profiling = opt.profiler_opt.cpu_profiling;
    let memory_profiling = opt.profiler_opt.memory_profiling;

    let mut cpu_profiler = handler.get_cpu_profiler();
    let mut memory_profiler = handler.get_mem_profiler();

    if cpu_profiling {
        let _cpu_start = cpu_profiler.start_profiling();
    }
    if memory_profiling {
        let _mem_start = memory_profiler.start_profiling();
    }

    match opt.block_executor_type {
        BlockExecutorTypeOpt::AptosVMWithBlockSTM => {
            run::<AptosVMBlockExecutor>(opt);
        },
        BlockExecutorTypeOpt::NativeVMWithBlockSTM => {
            run::<NativeVMBlockExecutor>(opt);
        },
        BlockExecutorTypeOpt::AptosVMParallelUncoordinated => {
            run::<AptosVMParallelUncoordinatedBlockExecutor>(opt);
        },
        BlockExecutorTypeOpt::NativeParallelUncoordinated => {
            run::<NativeParallelUncoordinatedBlockExecutor<NativeRawTransactionExecutor>>(opt);
        },
        BlockExecutorTypeOpt::NativeValueCacheParallelUncoordinated => {
            run::<NativeParallelUncoordinatedBlockExecutor<NativeValueCacheRawTransactionExecutor>>(
                opt,
            );
        },
        BlockExecutorTypeOpt::NativeNoStorageParallelUncoordinated => {
            run::<NativeParallelUncoordinatedBlockExecutor<NativeNoStorageRawTransactionExecutor>>(
                opt,
            );
        },
        BlockExecutorTypeOpt::PtxExecutor => {
            #[cfg(target_os = "linux")]
            ThreadManagerBuilder::set_thread_config_strategy(
                ThreadConfigStrategy::ThreadsPriority(48),
            );
            run::<PtxBlockExecutor>(opt);
        },
    }

    if cpu_profiling {
        let _cpu_end = cpu_profiler.end_profiling("");
    }
    if memory_profiling {
        let _mem_end = memory_profiler.end_profiling("./target/release/aptos-executor-benchmark");
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Opt::command().debug_assert()
}
