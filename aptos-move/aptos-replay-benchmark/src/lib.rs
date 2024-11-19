// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_types::{
    block_executor::{
        config::{
            BlockExecutorConfig, BlockExecutorConfigFromOnchain, BlockExecutorLocalConfig,
            BlockExecutorModuleCacheLocalConfig,
        },
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    on_chain_config::{FeatureFlag, Features, GasScheduleV2, OnChainConfig},
    state_store::{
        errors::StateviewError, state_key::StateKey, state_storage_usage::StateStorageUsage,
        state_value::StateValue, TStateView,
    },
    transaction::{
        signature_verified_transaction::{
            into_signature_verified_block, SignatureVerifiedTransaction,
        },
        Transaction, TransactionInfo, TransactionOutput, Version,
    },
};
use aptos_validator_interface::DebuggerStateView;
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, VMBlockExecutor};
use serde::Serialize;
use std::{collections::HashMap, sync::Mutex, time::Instant};

/// Config used by benchmarking.
fn block_execution_config(concurrency_level: usize) -> BlockExecutorConfig {
    BlockExecutorConfig {
        local: BlockExecutorLocalConfig {
            concurrency_level,
            allow_fallback: true,
            discard_failed_blocks: false,
            module_cache_config: BlockExecutorModuleCacheLocalConfig::default(),
        },
        // For replay, there is no block limit.
        onchain: BlockExecutorConfigFromOnchain::new_no_block_limit(),
    }
}

/// Returns the state key for on-chain config type.
fn config_state_key<T: OnChainConfig>() -> StateKey {
    StateKey::resource(T::address(), &T::struct_tag())
        .expect("Constructing state key for on-chain config must succeed")
}

/// Fetches the config from the storage, and modifies it based on the passed function. Panics if
/// there is a storage error, config does not exist or fails to (de-)serialize.
fn config_override<T: OnChainConfig + Serialize, F: FnOnce(&mut T)>(
    debugger_state_view: &DebuggerStateView,
    override_func: F,
) -> (StateKey, StateValue) {
    let state_key = config_state_key::<T>();
    let state_value = debugger_state_view
        .get_state_value(&state_key)
        .unwrap_or_else(|err| {
            panic!(
                "Failed to fetch on-chain config for {:?}: {:?}",
                state_key, err
            )
        })
        .unwrap_or_else(|| panic!("On-chain config for {:?} must always exist", state_key));

    let mut config = T::deserialize_into_config(state_value.bytes())
        .expect("On-chain config must be deserializable");
    override_func(&mut config);
    let config_bytes = bcs::to_bytes(&config).expect("On-chain config must be serializable");

    let new_state_value = state_value.map_bytes(|_| Ok(config_bytes.into())).unwrap();
    (state_key, new_state_value)
}

/// State view used for setting up the benchmarks. Maintains a set which caches execution reads,
/// populated during block generation phase. These reads are used to create a [BlockReadSet] later,
/// which is used as an input state for the benchmarking.
struct StateViewWithReadSet {
    /// Captured read-set.
    reads: Mutex<HashMap<StateKey, StateValue>>,
    /// Remote state view for the specified version.
    debugger_state_view: DebuggerStateView,
}

impl TStateView for StateViewWithReadSet {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &Self::Key) -> Result<Option<StateValue>, StateviewError> {
        // Check the read-set first.
        if let Some(state_value) = self.reads.lock().unwrap().get(state_key) {
            return Ok(Some(state_value.clone()));
        }

        // We do not allow failures because then benchmarking will not be correct (we miss a read).
        // Plus, these failures should not happen when replaying past transactions.
        let maybe_state_value = self
            .debugger_state_view
            .get_state_value(state_key)
            .unwrap_or_else(|err| {
                panic!("Failed to fetch state value for {:?}: {:?}", state_key, err)
            });

        // Populate the read-set if first access.
        let mut reads = self.reads.lock().unwrap();
        if !reads.contains_key(state_key) {
            if let Some(state_value) = &maybe_state_value {
                reads.insert(state_key.clone(), state_value.clone());
            }
        }
        drop(reads);

        Ok(maybe_state_value)
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateviewError> {
        unreachable!("Should not be called when benchmarking")
    }
}

/// Immutable read-set used as an input state for running a block of transactions.
struct BlockReadSet {
    data: HashMap<StateKey, StateValue>,
}

impl TStateView for BlockReadSet {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &Self::Key) -> Result<Option<StateValue>, StateviewError> {
        Ok(self.data.get(state_key).cloned())
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateviewError> {
        unreachable!("Should not be called when benchmarking")
    }
}

/// A workload to benchmark. Contains signature verified transactions, and metadata specifying the
/// start and end versions of these transactions.
struct Workload {
    txn_provider: DefaultTxnProvider<SignatureVerifiedTransaction>,
    transaction_slice_metadata: TransactionSliceMetadata,
}

impl Workload {
    /// Returns a new workload to execute transactions at specified version.
    fn new(begin: Version, txns: Vec<Transaction>) -> Self {
        let end = begin + txns.len() as Version;
        let transaction_slice_metadata = TransactionSliceMetadata::chunk(begin, end);

        let signature_verified_txns = into_signature_verified_block(txns);
        let txn_provider = DefaultTxnProvider::new(signature_verified_txns);

        Workload {
            txn_provider,
            transaction_slice_metadata,
        }
    }

    /// Returns the first transaction version in the workload.
    fn first_version(&self) -> Version {
        match &self.transaction_slice_metadata {
            TransactionSliceMetadata::Chunk { begin, .. } => *begin,
            _ => unreachable!("Transaction slice metadata is always a chunk"),
        }
    }

    /// Returns the last transaction version in the workload.
    fn last_version(&self) -> Version {
        match &self.transaction_slice_metadata {
            TransactionSliceMetadata::Chunk { end, .. } => *end - 1,
            _ => unreachable!("Transaction slice metadata is always a chunk"),
        }
    }
}

/// Captures information for benchmarking a single block of signature-verified transactions.
struct Block {
    /// Pre-execution state view to run the workload.
    input: BlockReadSet,
    /// Expected outputs of execution.
    output: Vec<TransactionOutput>,
    /// Workload containing transactions to execute.
    workload: Workload,
}

impl Block {
    /// Executes the workload using the provided executor and at specified concurrency level.
    #[inline(always)]
    fn execute(&self, executor: &AptosVMBlockExecutor, concurrency_level: usize) {
        executor
            .execute_block_with_config(
                &self.workload.txn_provider,
                &self.input,
                block_execution_config(concurrency_level),
                self.workload.transaction_slice_metadata,
            )
            .unwrap_or_else(|err| {
                panic!(
                    "Block execution should not fail, but returned an error: {:?}",
                    err
                )
            });
    }

    /// Generates a new [Block] for the specified workload. During the generation, the workload is
    /// pre-executed to determine the read-set, and the expected outputs.
    fn generate(
        workload: Workload,
        state_view: StateViewWithReadSet,
        concurrency_level: usize,
    ) -> anyhow::Result<Self> {
        let executor = AptosVMBlockExecutor::new();
        let output = executor
            .execute_block_with_config(
                &workload.txn_provider,
                &state_view,
                block_execution_config(concurrency_level),
                workload.transaction_slice_metadata,
            )
            .map_err(|err| anyhow!("Failed to generate block for benchmarking: {:?}", err))?
            .into_transaction_outputs_forced();

        let data = state_view.reads.into_inner().unwrap();
        Ok(Block {
            input: BlockReadSet { data },
            output,
            workload,
        })
    }
}

/// Represents a closed interval for transaction versions.
pub struct ClosedInterval {
    begin: Version,
    end: Version,
}

impl ClosedInterval {
    pub fn new(begin: Version, end: Version) -> Self {
        assert!(
            begin <= end,
            "Transaction versions are not a valid closed interval: [{}, {}].",
            begin,
            end,
        );
        Self { begin, end }
    }
}

/// Overrides for different environment configs, such as feature flags, etc.
#[derive(Debug)]
pub struct EnvironmentOverride {
    enable_features: Vec<FeatureFlag>,
    disable_features: Vec<FeatureFlag>,
    gas_feature_version: Option<u64>,
}

impl EnvironmentOverride {
    pub fn new(
        enable_features: Vec<FeatureFlag>,
        disable_features: Vec<FeatureFlag>,
        gas_feature_version: Option<u64>,
    ) -> Self {
        assert!(
            enable_features
                .iter()
                .all(|f| !disable_features.contains(f)),
            "Enable and disable feature flags cannot overlap"
        );

        Self {
            enable_features,
            disable_features,
            gas_feature_version,
        }
    }

    fn generate_state_override(
        &self,
        debugger_state_view: &DebuggerStateView,
    ) -> HashMap<StateKey, StateValue> {
        let mut state_override = HashMap::new();

        // Enable/disable features.
        let (features_state_key, features_state_value) =
            config_override::<Features, _>(debugger_state_view, |features| {
                for feature in &self.enable_features {
                    if features.is_enabled(*feature) {
                        println!("[WARN] Feature {:?} is already enabled", feature)
                    }
                    features.enable(*feature);
                }
                for feature in &self.disable_features {
                    if !features.is_enabled(*feature) {
                        println!("[WARN] Feature {:?} is already disabled", feature)
                    }
                    features.disable(*feature);
                }
            });
        state_override.insert(features_state_key, features_state_value);

        // Override gas feature version.
        if let Some(gas_feature_version_override) = self.gas_feature_version {
            let (gas_schedule_v2_state_key, gas_schedule_v2_state_value) =
                config_override::<GasScheduleV2, _>(debugger_state_view, |gas_schedule_v2| {
                    gas_schedule_v2.feature_version = gas_feature_version_override;
                });
            state_override.insert(gas_schedule_v2_state_key, gas_schedule_v2_state_value);
        }

        state_override
    }

    /// Checks if the output of the transaction contains writes to overridden configs. If so,
    /// warnings are logged to stdout.
    fn ensure_overrides_do_not_conflict(&self, version: Version, output: &TransactionOutput) {
        let features_state_key = config_state_key::<Features>();
        let gas_schedule_v2_state_key = config_state_key::<GasScheduleV2>();

        for (state_key, _) in output.write_set() {
            if state_key == &features_state_key {
                println!(
                    "[WARN] Features are being updated by transaction {}",
                    version
                );
            }

            if self.gas_feature_version.is_some() && state_key == &gas_schedule_v2_state_key {
                println!(
                    "[WARN] Gas schedule V2 is being updated by transaction {}",
                    version
                );
            }
        }
    }
}

pub struct AptosBenchmarkRunner {
    /// Used to fetch transactions and transaction infos from the DB or REST endpoint.
    debugger: AptosDebugger,
    /// Specifies the closed interval of transaction versions to execute.
    versions: ClosedInterval,
    /// Different concurrency levels to benchmark.
    concurrency_levels: Vec<usize>,
    /// Number of times benchmark is repeated for each concurrency level.
    num_repeats: usize,
    /// Specifies how to override execution environment configs for each benchmark.
    environment_override: EnvironmentOverride,
}

impl AptosBenchmarkRunner {
    pub fn new(
        debugger: AptosDebugger,
        versions: ClosedInterval,
        concurrency_levels: Vec<usize>,
        num_repeats: Option<usize>,
        environment_override: EnvironmentOverride,
    ) -> Self {
        assert!(
            !concurrency_levels.is_empty(),
            "At least one concurrency level must be provided"
        );

        let default_num_repeats = 3;
        let num_repeats = num_repeats.unwrap_or_else(|| {
            println!(
                "[WARN] Using default number of repeats: {}",
                default_num_repeats
            );
            default_num_repeats
        });
        assert!(
            num_repeats >= default_num_repeats,
            "Number of times to repeat the benchmark should be at least the default value {}",
            default_num_repeats
        );

        Self {
            debugger,
            versions,
            concurrency_levels,
            num_repeats,
            environment_override,
        }
    }

    /// Creates [StateViewWithReadSet] to generate the workloads for benchmarking. Also, overrides
    /// on-chain configs based on the specified environment.
    fn state_with_at_version_with_override(&self, version: Version) -> StateViewWithReadSet {
        let debugger_state_view = self.debugger.state_view_at_version(version);
        let state_override = self
            .environment_override
            .generate_state_override(&debugger_state_view);

        StateViewWithReadSet {
            reads: Mutex::new(state_override),
            debugger_state_view,
        }
    }

    /// Generates a single [Block] for benchmarking.
    fn generate_block(&self, begin: Version, txns: Vec<Transaction>) -> anyhow::Result<Block> {
        // To generate blocks, run with maximum concurrency specified.
        let concurrency_level = *self
            .concurrency_levels
            .iter()
            .max()
            .expect("At least one concurrency level must be provided");

        let workload = Workload::new(begin, txns);
        let state_view = self.state_with_at_version_with_override(begin);

        Block::generate(workload, state_view, concurrency_level)
    }

    /// Generates a sequence of [Block]s for benchmarking. Block execution boundaries correspond to
    /// the real boundaries on-chain.
    async fn generate_blocks(&self, txns: Vec<Transaction>) -> anyhow::Result<Vec<Block>> {
        let mut blocks = Vec::with_capacity(txns.len());

        let mut curr_block = Vec::with_capacity(txns.len());
        let mut curr_version = self.versions.begin;

        for txn in txns {
            if txn.is_block_start() && !curr_block.is_empty() {
                let block_size = curr_block.len() as Version;
                blocks.push(self.generate_block(curr_version, std::mem::take(&mut curr_block))?);
                curr_version += block_size;
            }
            curr_block.push(txn);
        }

        if !curr_block.is_empty() {
            blocks.push(self.generate_block(curr_version, curr_block)?);
        }

        Ok(blocks)
    }

    /// Checks generated [Block]s against the on-chain data:
    ///   - Outputs should match on-chain transaction infos.
    ///   - Outputs should not write to overridden configs.
    fn check_blocks(&self, blocks: &[Block], txn_infos: &[TransactionInfo]) -> anyhow::Result<()> {
        for (idx, (output, txn_info)) in blocks
            .iter()
            .flat_map(|b| &b.output)
            .zip(txn_infos)
            .enumerate()
        {
            let version = self.versions.begin + idx as Version;
            if let Err(err) = output.ensure_match_transaction_info(version, txn_info, None, None) {
                println!("[WARN] Output mismatch: {:?}", err);
            }

            self.environment_override
                .ensure_overrides_do_not_conflict(version, output);
        }
        Ok(())
    }

    /// Logs different statistics about [Block]s:
    fn analyze_blocks(&self, blocks: &[Block]) {
        for (idx, block) in blocks.iter().enumerate() {
            let num_txns = block.workload.txn_provider.get_txns().len();
            println!(
                "Block {}: versions [{}, {}] with {} transactions",
                idx + 1,
                block.workload.first_version(),
                block.workload.last_version(),
                num_txns
            )
        }
    }

    /// The main entrypoint for benchmarking: for specified concurrency levels, replays blocks of
    /// transactions and measures the overall time taken. Note that during execution each block
    /// runs on its own state, so the execution time does not take into account block commit.
    pub async fn benchmark_past_transactions(&self) -> anyhow::Result<()> {
        let limit = self.versions.end - self.versions.begin + 1;
        let (txns, txn_infos) = self
            .debugger
            .get_committed_transactions(self.versions.begin, limit)
            .await?;

        println!("Generating blocks for benchmarking ...");
        let blocks = self.generate_blocks(txns).await?;

        println!("Checking generated blocks ...");
        self.check_blocks(&blocks, &txn_infos)?;

        println!("Analyzing {} generated blocks ...", blocks.len());
        self.analyze_blocks(&blocks);

        println!("Benchmarking ... \n");

        for concurrency_level in &self.concurrency_levels {
            println!("Concurrency level: {}", concurrency_level);
            let mut times = Vec::with_capacity(self.num_repeats);

            for i in 0..self.num_repeats {
                let start_time = Instant::now();

                let executor = AptosVMBlockExecutor::new();
                for block in &blocks {
                    block.execute(&executor, *concurrency_level);
                }

                let time = start_time.elapsed().as_millis();
                println!(
                    "[{}/{}] Execution time is {}ms",
                    i + 1,
                    self.num_repeats,
                    time,
                );
                times.push(time);
            }
            times.sort();

            println!(
                "Median execution time is {}ms\n",
                times[self.num_repeats / 2],
            );
        }

        Ok(())
    }
}
