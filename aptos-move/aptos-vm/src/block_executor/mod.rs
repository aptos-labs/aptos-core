// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod vm_wrapper;

use crate::{
    block_executor::vm_wrapper::AptosExecutorTask,
    counters::{BLOCK_EXECUTOR_CONCURRENCY, BLOCK_EXECUTOR_EXECUTE_BLOCK_SECONDS},
};
use aptos_aggregator::{
    delayed_change::DelayedChange, delta_change_set::DeltaOp, resolver::TAggregatorV1View,
};
use aptos_block_executor::{
    code_cache_global::ImmutableModuleCache, errors::BlockExecutionError, executor::BlockExecutor,
    task::TransactionOutput as BlockExecutorTransactionOutput,
    txn_commit_hook::TransactionCommitHook, types::InputOutputKey,
};
use aptos_infallible::Mutex;
use aptos_types::{
    block_executor::config::BlockExecutorConfig,
    contract_event::ContractEvent,
    error::PanicError,
    fee_statement::FeeStatement,
    state_store::{state_key::StateKey, state_value::StateValueMetadata, StateView, StateViewId},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, BlockOutput,
        TransactionOutput, TransactionStatus,
    },
    vm::modules::AptosModuleExtension,
    write_set::WriteOp,
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::{flush_speculative_logs, init_speculative_logs};
use aptos_vm_types::{
    abstract_write_op::AbstractResourceWriteOp, module_write_set::ModuleWrite, output::VMOutput,
    resolver::ResourceGroupSize,
};
use move_binary_format::{errors::Location, CompiledModule};
use move_core_types::{
    language_storage::{ModuleId, StructTag},
    value::MoveTypeLayout,
    vm_status::{StatusCode, VMStatus},
};
use move_vm_runtime::{Module, WithRuntimeEnvironment};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use once_cell::sync::{Lazy, OnceCell};
use std::{
    collections::{BTreeMap, HashSet},
    hash::Hash,
    ops::Deref,
    sync::Arc,
};

static RAYON_EXEC_POOL: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .thread_name(|index| format!("par_exec-{}", index))
            .build()
            .unwrap(),
    )
});

/// Immutable global module cache that can be shared across multiple block executions. The size of
/// the cache is fixed within a single block (modules are not inserted or removed) and it is only
/// mutated at the block boundaries. Do not use if multiple blocks are executed concurrently.
static GLOBAL_MODULE_CACHE: Lazy<
    Arc<ImmutableModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension>>,
> = Lazy::new(|| Arc::new(ImmutableModuleCache::empty()));

/// The maximum size of struct name index map in runtime environment. Checked at block boundaries
/// only.
const MAX_STRUCT_NAME_INDEX_MAP_SIZE: usize = 100_000;

/// A cached environment that can be persisted globally across blocks.
static GLOBAL_ENVIRONMENT: Lazy<Mutex<Option<AptosEnvironment>>> = Lazy::new(|| Mutex::new(None));

/// Returns the cached environment if it exists and has the same configuration as if it was
/// created based on the current state, or creates a new one and caches it. Should only be
/// called at the block boundaries.
fn get_environment_with_delayed_field_optimization_enabled<K, DC, VC, E>(
    state_view: &impl StateView,
    global_module_cache: &ImmutableModuleCache<K, DC, VC, E>,
) -> Result<AptosEnvironment, VMStatus>
where
    K: Hash + Eq + Clone,
    VC: Deref<Target = Arc<DC>>,
{
    // Create a new environment.
    let current_env = AptosEnvironment::new_with_delayed_field_optimization_enabled(state_view);

    // Lock the cache, and check if the environment is the same.
    let mut global_environment = GLOBAL_ENVIRONMENT.lock();
    if let Some(previous_env) = global_environment.as_ref() {
        if &current_env == previous_env {
            let runtime_env = previous_env.runtime_environment();
            let struct_name_index_map_size = runtime_env
                .struct_name_index_map_size()
                .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
            if struct_name_index_map_size > MAX_STRUCT_NAME_INDEX_MAP_SIZE {
                // Cache is too large, flush it. Also flush the module cache.
                runtime_env.flush_struct_name_and_info_caches();
                global_module_cache.flush_unchecked();
            }
            return Ok(previous_env.clone());
        }
    }

    // It is not cached or has changed, so we have to reset it. As a result, we need to flush
    // the cross-block cache because we need to reload all modules with new configs.
    *global_environment = Some(current_env.clone());
    drop(global_environment);
    global_module_cache.flush_unchecked();

    Ok(current_env)
}

/// Output type wrapper used by block executor. VM output is stored first, then
/// transformed into TransactionOutput type that is returned.
#[derive(Debug)]
pub struct AptosTransactionOutput {
    // Note: should these mutexes be changed to ExplicitSyncSwapper?
    vm_output: Mutex<Option<VMOutput>>,
    committed_output: OnceCell<TransactionOutput>,
}

impl AptosTransactionOutput {
    pub(crate) fn new(output: VMOutput) -> Self {
        Self {
            vm_output: Mutex::new(Some(output)),
            committed_output: OnceCell::new(),
        }
    }

    pub(crate) fn committed_output(&self) -> &TransactionOutput {
        self.committed_output.get().unwrap()
    }

    fn take_output(mut self) -> TransactionOutput {
        match self.committed_output.take() {
            Some(output) => output,
            // TODO: revisit whether we should always get it via committed, or o.w. create a
            // dedicated API without creating empty data structures.
            // This is currently used because we do not commit skip_output() transactions.
            None => self
                .vm_output
                .lock()
                .take()
                .expect("Output must be set")
                .into_transaction_output()
                .expect("Transaction output is not alerady materialized"),
        }
    }
}

impl BlockExecutorTransactionOutput for AptosTransactionOutput {
    type Txn = SignatureVerifiedTransaction;

    /// Execution output for transactions that comes after SkipRest signal or when there was a
    /// problem creating the output (e.g. group serialization issue).
    fn skip_output() -> Self {
        Self::new(VMOutput::empty_with_status(TransactionStatus::Retry))
    }

    fn discard_output(discard_code: StatusCode) -> Self {
        Self::new(VMOutput::empty_with_status(TransactionStatus::Discard(
            discard_code,
        )))
    }

    // TODO: get rid of the cloning data-structures in the following APIs.

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn resource_group_write_set(
        &self,
    ) -> Vec<(
        StateKey,
        WriteOp,
        ResourceGroupSize,
        BTreeMap<StructTag, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
    )> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get resource group writes")
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| {
                if let AbstractResourceWriteOp::WriteResourceGroup(group_write) = write {
                    Some((
                        key.clone(),
                        group_write.metadata_op().clone(),
                        group_write
                            .maybe_group_op_size()
                            .unwrap_or(ResourceGroupSize::zero_combined()),
                        group_write
                            .inner_ops()
                            .iter()
                            .map(|(tag, (op, maybe_layout))| {
                                (tag.clone(), (op.clone(), maybe_layout.clone()))
                            })
                            .collect(),
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    /// More efficient implementation to avoid unnecessarily cloning inner_ops.
    fn resource_group_metadata_ops(&self) -> Vec<(StateKey, WriteOp)> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get metadata ops")
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| {
                if let AbstractResourceWriteOp::WriteResourceGroup(group_write) = write {
                    Some((key.clone(), group_write.metadata_op().clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn resource_write_set(&self) -> Vec<(StateKey, Arc<WriteOp>, Option<Arc<MoveTypeLayout>>)> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get resource writes")
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| match write {
                AbstractResourceWriteOp::Write(write_op) => {
                    Some((key.clone(), Arc::new(write_op.clone()), None))
                },
                AbstractResourceWriteOp::WriteWithDelayedFields(write) => Some((
                    key.clone(),
                    Arc::new(write.write_op.clone()),
                    Some(write.layout.clone()),
                )),
                _ => None,
            })
            .collect()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn module_write_set(&self) -> BTreeMap<StateKey, ModuleWrite<WriteOp>> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get module writes")
            .module_write_set()
            .clone()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn aggregator_v1_write_set(&self) -> BTreeMap<StateKey, WriteOp> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get aggregator V1 writes")
            .aggregator_v1_write_set()
            .clone()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn aggregator_v1_delta_set(&self) -> Vec<(StateKey, DeltaOp)> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get deltas")
            .aggregator_v1_delta_set()
            .iter()
            .map(|(key, op)| (key.clone(), *op))
            .collect()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get aggregator change set")
            .delayed_field_change_set()
            .clone()
    }

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(StateKey, StateValueMetadata, Arc<MoveTypeLayout>)> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output to be set to get reads")
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| {
                if let AbstractResourceWriteOp::InPlaceDelayedFieldChange(change) = write {
                    Some((key.clone(), change.metadata.clone(), change.layout.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    fn group_reads_needing_delayed_field_exchange(&self) -> Vec<(StateKey, StateValueMetadata)> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output to be set to get reads")
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| {
                if let AbstractResourceWriteOp::ResourceGroupInPlaceDelayedFieldChange(change) =
                    write
                {
                    Some((key.clone(), change.metadata.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn get_events(&self) -> Vec<(ContractEvent, Option<MoveTypeLayout>)> {
        self.vm_output
            .lock()
            .as_ref()
            .expect("Output must be set to get events")
            .events()
            .to_vec()
    }

    fn materialize_agg_v1(&self, view: &impl TAggregatorV1View<Identifier = StateKey>) {
        self.vm_output
            .lock()
            .as_mut()
            .expect("Output must be set to incorporate materialized data")
            .try_materialize(view)
            .expect("Delta materialization failed");
    }

    fn incorporate_materialized_txn_output(
        &self,
        aggregator_v1_writes: Vec<(StateKey, WriteOp)>,
        materialized_resource_write_set: Vec<(StateKey, WriteOp)>,
        materialized_events: Vec<ContractEvent>,
    ) -> Result<(), PanicError> {
        assert!(
            self.committed_output
                .set(
                    self.vm_output
                        .lock()
                        .take()
                        .expect("Output must be set to incorporate materialized data")
                        .into_transaction_output_with_materialized_write_set(
                            aggregator_v1_writes,
                            materialized_resource_write_set,
                            materialized_events,
                        )?,
                )
                .is_ok(),
            "Could not combine VMOutput with the materialized resource and event data"
        );
        Ok(())
    }

    fn set_txn_output_for_non_dynamic_change_set(&self) {
        assert!(
            self.committed_output
                .set(
                    self.vm_output
                        .lock()
                        .take()
                        .expect("Output must be set to incorporate materialized data")
                        .into_transaction_output()
                        .expect("We should be able to always convert to transaction output"),
                )
                .is_ok(),
            "Could not combine VMOutput with the materialized resource and event data"
        );
    }

    /// Return the fee statement of the transaction.
    /// Should never be called after vm_output is consumed.
    fn fee_statement(&self) -> FeeStatement {
        *self
            .vm_output
            .lock()
            .as_ref()
            .expect("Output to be set to get fee statement")
            .fee_statement()
    }

    fn output_approx_size(&self) -> u64 {
        let vm_output = self.vm_output.lock();
        vm_output
            .as_ref()
            .expect("Output to be set to get approximate size")
            .materialized_size()
    }

    fn get_write_summary(&self) -> HashSet<InputOutputKey<StateKey, StructTag, DelayedFieldID>> {
        let vm_output = self.vm_output.lock();
        let output = vm_output
            .as_ref()
            .expect("Output to be set to get write summary");

        let mut writes = HashSet::new();

        for (state_key, write) in output.resource_write_set() {
            match write {
                AbstractResourceWriteOp::Write(_)
                | AbstractResourceWriteOp::WriteWithDelayedFields(_) => {
                    writes.insert(InputOutputKey::Resource(state_key.clone()));
                },
                AbstractResourceWriteOp::WriteResourceGroup(write) => {
                    for tag in write.inner_ops().keys() {
                        writes.insert(InputOutputKey::Group(state_key.clone(), tag.clone()));
                    }
                },
                AbstractResourceWriteOp::InPlaceDelayedFieldChange(_)
                | AbstractResourceWriteOp::ResourceGroupInPlaceDelayedFieldChange(_) => {
                    // No conflicts on resources from in-place delayed field changes.
                    // Delayed fields conflicts themselves are handled via
                    // delayed_field_change_set below.
                },
            }
        }

        for identifier in output.delayed_field_change_set().keys() {
            writes.insert(InputOutputKey::DelayedField(*identifier));
        }

        writes
    }
}

pub struct BlockAptosVM;

impl BlockAptosVM {
    fn execute_block_on_thread_pool<
        S: StateView + Sync,
        L: TransactionCommitHook<Output = AptosTransactionOutput>,
    >(
        executor_thread_pool: Arc<rayon::ThreadPool>,
        signature_verified_block: &[SignatureVerifiedTransaction],
        state_view: &S,
        global_module_cache: Arc<
            ImmutableModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension>,
        >,
        config: BlockExecutorConfig,
        transaction_commit_listener: Option<L>,
    ) -> Result<BlockOutput<TransactionOutput>, VMStatus> {
        let _timer = BLOCK_EXECUTOR_EXECUTE_BLOCK_SECONDS.start_timer();
        let num_txns = signature_verified_block.len();
        if state_view.id() != StateViewId::Miscellaneous {
            // Speculation is disabled in Miscellaneous context, which is used by testing and
            // can even lead to concurrent execute_block invocations, leading to errors on flush.
            init_speculative_logs(num_txns);
        }

        BLOCK_EXECUTOR_CONCURRENCY.set(config.local.concurrency_level as i64);

        let environment = get_environment_with_delayed_field_optimization_enabled(
            state_view,
            global_module_cache.as_ref(),
        )?;

        let executor = BlockExecutor::<SignatureVerifiedTransaction, AptosExecutorTask, S, L>::new(
            config,
            executor_thread_pool,
            global_module_cache,
            transaction_commit_listener,
        );

        let ret = executor.execute_block(environment, signature_verified_block, state_view);
        match ret {
            Ok(block_output) => {
                let (transaction_outputs, block_end_info) = block_output.into_inner();
                let output_vec: Vec<_> = transaction_outputs
                    .into_iter()
                    .map(|output| output.take_output())
                    .collect();

                // Flush the speculative logs of the committed transactions.
                let pos = output_vec.partition_point(|o| !o.status().is_retry());

                if state_view.id() != StateViewId::Miscellaneous {
                    // Speculation is disabled in Miscellaneous context, which is used by testing and
                    // can even lead to concurrent execute_block invocations, leading to errors on flush.
                    flush_speculative_logs(pos);
                }

                Ok(BlockOutput::new(output_vec, block_end_info))
            },
            Err(BlockExecutionError::FatalBlockExecutorError(PanicError::CodeInvariantError(
                err_msg,
            ))) => Err(VMStatus::Error {
                status_code: StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR,
                sub_status: None,
                message: Some(err_msg),
            }),
            Err(BlockExecutionError::FatalVMError(err)) => Err(err),
        }
    }

    pub fn execute_block_on_thread_pool_without_global_module_cache<
        S: StateView + Sync,
        L: TransactionCommitHook<Output = AptosTransactionOutput>,
    >(
        executor_thread_pool: Arc<rayon::ThreadPool>,
        signature_verified_block: &[SignatureVerifiedTransaction],
        state_view: &S,
        config: BlockExecutorConfig,
        transaction_commit_listener: Option<L>,
    ) -> Result<BlockOutput<TransactionOutput>, VMStatus> {
        Self::execute_block_on_thread_pool::<S, L>(
            executor_thread_pool,
            signature_verified_block,
            state_view,
            Arc::new(ImmutableModuleCache::empty()),
            config,
            transaction_commit_listener,
        )
    }

    /// Uses shared thread pool and shared global module cache to execute blocks.
    pub fn execute_block<
        S: StateView + Sync,
        L: TransactionCommitHook<Output = AptosTransactionOutput>,
    >(
        signature_verified_block: &[SignatureVerifiedTransaction],
        state_view: &S,
        config: BlockExecutorConfig,
        transaction_commit_listener: Option<L>,
    ) -> Result<BlockOutput<TransactionOutput>, VMStatus> {
        Self::execute_block_on_thread_pool::<S, L>(
            Arc::clone(&RAYON_EXEC_POOL),
            signature_verified_block,
            state_view,
            Arc::clone(&GLOBAL_MODULE_CACHE),
            config,
            transaction_commit_listener,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_block_executor::code_cache_global::ImmutableModuleCache;
    use aptos_language_e2e_tests::data_store::FakeDataStore;
    use aptos_types::on_chain_config::{FeatureFlag, Features};
    use aptos_vm_environment::environment::AptosEnvironment;
    use claims::assert_ok;
    use move_vm_types::code::mock_verified_code;

    #[test]
    fn test_cross_block_module_cache_flush() {
        let global_module_cache = ImmutableModuleCache::empty();

        global_module_cache.insert(0, mock_verified_code(0, None));
        assert_eq!(global_module_cache.size(), 1);

        global_module_cache.flush_unchecked();
        assert_eq!(global_module_cache.size(), 0);

        // Now check that cache is flushed when the environment is flushed.
        let mut state_view = FakeDataStore::default();
        let env_old = AptosEnvironment::new_with_delayed_field_optimization_enabled(&state_view);

        for i in 0..10 {
            global_module_cache.insert(i, mock_verified_code(i, None));
        }
        assert_eq!(global_module_cache.size(), 10);

        let mut features = Features::default();
        features.disable(FeatureFlag::KEYLESS_ACCOUNTS);
        state_view.set_features(features);

        // New environment means we need to also flush global caches - to invalidate struct name
        // indices.
        let env_new = assert_ok!(get_environment_with_delayed_field_optimization_enabled(
            &state_view,
            &global_module_cache,
        ));
        assert!(env_old != env_new);
        assert_eq!(global_module_cache.size(), 0);
    }
}
