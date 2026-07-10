// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    code_cache_global::{add_module_write_to_module_cache, GlobalModuleCache},
    errors::ParallelBlockExecutionError,
    explicit_sync_wrapper::ExplicitSyncWrapper,
    limit_processor::BlockGasLimitProcessor,
    record::{Record, RecordStatus},
    scheduler_wrapper::SchedulerWrapper,
    types::ReadWriteSummary,
    view::ViewArgs,
};
use aptos_logger::error;
use aptos_mvhashmap::{types::TxnIndex, MVHashMap};
use aptos_types::{
    error::{code_invariant_error, PanicError, PanicOr},
    on_chain_config::BlockGasLimitType,
    state_store::TStateView,
    transaction::BlockExecutableTransaction as Transaction,
    vm::modules::AptosModuleExtension,
};
use aptos_vm_environment::environment::AptosEnvironment;
use crossbeam::utils::CachePadded;
use fail::fail_point;
use move_binary_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use move_vm_runtime::{Module, RuntimeEnvironment};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use parking_lot::Mutex;
use std::{
    collections::{BTreeSet, HashSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// A stored record together with the store-level state the commit path
/// manages: the skip-rest override (set when the block is cut, since the
/// record itself is shared immutably) and the facts computed once when the
/// record was stored.
struct RecordEntry<R: Record> {
    /// The record is behind an `Arc` so that validation can clone it out and
    /// validate without holding the slot lock, concurrently with a newer
    /// incarnation replacing the entry.
    record: Arc<R>,
    /// Whether the commit path turned this (successful) record into a
    /// skip-rest one because a block limit was reached.
    forced_skip_rest: bool,
    maybe_read_write_summary: Option<ReadWriteSummary<R::Key, R::Tag>>,
    maybe_approx_output_size: Option<u64>,
}

impl<R: Record> RecordEntry<R> {
    fn check_success_or_skip_status(&self) -> Result<&Arc<R>, PanicError> {
        if !self.record.status().is_success_or_skip_rest() {
            return Err(code_invariant_error(format!(
                "Output status {:?}!= success or skip rest",
                self.record.status()
            )));
        }
        Ok(&self.record)
    }
}

/// The store of the records of the latest completed incarnation of each
/// transaction in the block (with one extra slot for the block epilogue txn).
pub struct Records<R: Record> {
    records: Vec<CachePadded<Mutex<Option<RecordEntry<R>>>>>,
    // Used to record if the latest incarnation of a txn was a failure due to the
    // speculative nature of parallel execution.
    speculative_failures: Vec<CachePadded<AtomicBool>>,
}

impl<R: Record> Records<R> {
    /// num_txns passed here is typically larger than the number of txns in the block,
    /// currently by 1 to account for the block epilogue txn.
    pub fn new(num_txns: TxnIndex) -> Self {
        Self {
            records: (0..num_txns)
                .map(|_| CachePadded::new(Mutex::new(None)))
                .collect(),
            speculative_failures: (0..num_txns)
                .map(|_| CachePadded::new(AtomicBool::new(false)))
                .collect(),
        }
    }

    pub(crate) fn record(
        &self,
        txn_idx: TxnIndex,
        record: R,
        block_gas_limit_type: &BlockGasLimitType,
        user_txn_bytes_len: u64,
    ) -> Result<(), PanicError> {
        // The commit facts are computed once, only for outputs that can commit.
        let (maybe_approx_output_size, maybe_read_write_summary) =
            if record.status().is_success_or_skip_rest() {
                let maybe_approx_output_size = block_gas_limit_type
                    .block_output_limit()
                    .map(|_| {
                        Ok::<_, PanicError>(
                            record.output_approx_size()?
                                + if block_gas_limit_type.include_user_txn_size_in_block_output() {
                                    user_txn_bytes_len
                                } else {
                                    0
                                },
                        )
                    })
                    .transpose()?;

                let maybe_read_write_summary = block_gas_limit_type
                    .conflict_penalty_window()
                    .map(|_| {
                        Ok::<_, PanicError>(ReadWriteSummary::new(
                            record.read_summary(),
                            record.write_summary()?,
                        ))
                    })
                    .transpose()?;

                (maybe_approx_output_size, maybe_read_write_summary)
            } else {
                (None, None)
            };

        self.speculative_failures[txn_idx as usize].store(false, Ordering::Relaxed);
        *self.records[txn_idx as usize].lock() = Some(RecordEntry {
            record: Arc::new(record),
            forced_skip_rest: false,
            maybe_read_write_summary,
            maybe_approx_output_size,
        });

        Ok(())
    }

    pub(crate) fn record_speculative_failure(&self, txn_idx: TxnIndex) {
        self.speculative_failures[txn_idx as usize].store(true, Ordering::Relaxed);
    }

    pub(crate) fn is_speculative_failure(&self, txn_idx: TxnIndex) -> bool {
        self.speculative_failures[txn_idx as usize].load(Ordering::Relaxed)
    }

    /// Returns the record of the latest completed incarnation of the txn, if
    /// one was stored. The record is cloned out so that the caller (e.g.
    /// validation) does not hold the slot lock while using it.
    pub(crate) fn get_record(&self, txn_idx: TxnIndex) -> Option<Arc<R>> {
        Some(Arc::clone(
            &self.records[txn_idx as usize].lock().as_ref()?.record,
        ))
    }

    // Should be called when txn_idx is committed, while holding commit lock.
    //
    // Records fee statement separately for block epilogue txn. This is done because the
    // recorded output will be taken by materialization which can be concurrent with the
    // block epilogue txn.
    //
    // Returns whether the block epilogue txn should be created. This is true when both
    // of the following conditions hold:
    // (1) the last txn in the block was committed (if any txns are left over, they must
    // all be skipped), and
    // (2) the last txn did not emit a new epoch event.
    // To avoid unnecessarily inspecting events, we only check (2) if (1) is true.
    pub(crate) fn commit(
        &self,
        txn_idx: TxnIndex,
        num_txns: TxnIndex,
        num_workers: usize,
        block_limit_processor: &mut BlockGasLimitProcessor<
            <R::Txn as Transaction>::Key,
            R::Key,
            R::Tag,
        >,
        maybe_block_epilogue_txn_idx: &ExplicitSyncWrapper<Option<TxnIndex>>,
        scheduler: &SchedulerWrapper,
    ) -> Result<(), PanicOr<ParallelBlockExecutionError>> {
        let mut entry_guard = self.records[txn_idx as usize].lock();

        // Transaction cannot be committed with below statuses, as:
        // - Speculative error must have failed validation.
        // - Execution w. delayed field code error propagates the error directly
        // and does not finish execution. Similar for FatalVMError / abort.
        // - A missing record means there is no output to commit.
        // check_success_or_skip_status below returns an invariant error for all
        // these cases, but we handle Abort case separately first.
        if let Some(entry) = entry_guard.as_ref() {
            if let RecordStatus::Abort(err) = entry.record.status() {
                // Fatal VM error.
                error!(
                    "FatalVMError from parallel execution {:?} at txn {}",
                    err, txn_idx
                );
                return Err(PanicOr::Or(ParallelBlockExecutionError::FatalVMError));
            }
        }
        let entry = entry_guard.as_mut().ok_or_else(|| {
            code_invariant_error(format!(
                "Recorded output not found at commit for txn {}",
                txn_idx
            ))
        })?;
        let record = Arc::clone(entry.check_success_or_skip_status()?);

        let maybe_read_write_summary = entry.maybe_read_write_summary.take();
        let has_new_epoch_event = record.has_new_epoch_event()?;

        let (mut skips_rest, mut must_create_epilogue_txn) =
            if entry.forced_skip_rest || matches!(record.status(), RecordStatus::SkipRest) {
                (true, !has_new_epoch_event)
            } else {
                assert!(matches!(record.status(), RecordStatus::Success));
                (false, txn_idx == num_txns - 1 && !has_new_epoch_event)
            };
        let fee_statement = record.fee_statement()?;

        // For committed txns, calculate the accumulated gas costs.
        block_limit_processor.accumulate_fee_statement(
            fee_statement,
            maybe_read_write_summary,
            entry.maybe_approx_output_size,
        );
        if block_limit_processor.is_hot_state_accumulation_enabled() {
            record.accumulate_hot_state(block_limit_processor)?;
        }

        if txn_idx < num_txns - 1
            && block_limit_processor.should_end_block_parallel()
            && !skips_rest
        {
            if matches!(record.status(), RecordStatus::Success) {
                must_create_epilogue_txn |= !has_new_epoch_event;
                entry.forced_skip_rest = true;
            }
            skips_rest = true;
        }

        drop(entry_guard);

        // Add before halt, so SchedulerV2 can organically observe and process post commit
        // processing tasks even after it has halted.
        scheduler.add_to_post_commit(txn_idx)?;

        // !!! CAUTION !!! after the txn_idx is added to the post commit queue, it is no longer
        // safe to expect an output be stored for txn_idx: post-commit materialization takes
        // the output (instead of cloning for efficiency) for parallel post-processing.

        // While panic errors can lead to halting parallel execution (and fallback),
        // below we may halt the execution by design (no errors) in cases when:
        // a) all transactions are scheduled for committing, or
        // b) we skip_rest after a transaction
        // Either all txn committed, or a committed txn caused an early halt.
        if (txn_idx + 1 == num_txns || skips_rest) && scheduler.halt() {
            block_limit_processor.finish_parallel_update_counters_and_log_info(
                txn_idx + 1,
                num_txns,
                num_workers,
            );

            // failpoint triggering error at the last committed transaction,
            // to test that next transaction is handled correctly
            fail_point!("commit-all-halt-err", |_| Err(code_invariant_error(
                "fail points: Last committed transaction halted"
            )
            .into()));
        }

        if must_create_epilogue_txn {
            *maybe_block_epilogue_txn_idx.acquire().dereference_mut() = Some(txn_idx + 1);
        }

        Ok(())
    }

    pub(crate) fn for_each_resource_key_no_aggregator_v1(
        &self,
        txn_idx: TxnIndex,
        mut callback: impl FnMut(&R::Key) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        if let Some(record) = self.get_record(txn_idx) {
            record.for_each_resource_key(&mut callback)?;
        }
        Ok(())
    }

    /// Returns an error if callback returns an error.
    pub(crate) fn for_each_resource_group_key_and_tags(
        &self,
        txn_idx: TxnIndex,
        mut callback: impl FnMut(&R::Key, HashSet<&R::Tag>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        if let Some(record) = self.get_record(txn_idx) {
            record.for_each_resource_group_key_and_tags(&mut callback)?;
        }
        Ok(())
    }

    pub(crate) fn modified_group_key_and_tags_cloned(
        &self,
        txn_idx: TxnIndex,
    ) -> Vec<(R::Key, HashSet<R::Tag>)> {
        self.get_record(txn_idx).map_or_else(Vec::new, |record| {
            record.resource_group_tags().expect("Output must be set")
        })
    }

    // Extracts a set of resource paths (keys) written or updated during execution from
    // transaction output. The group keys are not included.
    pub(crate) fn modified_resource_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = R::Key>> {
        let record = self.record_if_success_or_skip_rest(txn_idx)?;
        Some(
            record
                .resource_write_set()
                .expect("Output must be set")
                .into_keys(),
        )
    }

    // The output needs to be Success or SkipRest, o.w. invariant error is returned.
    pub(crate) fn publish_module_write_set(
        &self,
        txn_idx: TxnIndex,
        global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        versioned_cache: &MVHashMap<R::Key, R::Tag, R::Value, DelayedFieldID>,
        runtime_environment: &RuntimeEnvironment,
        scheduler: &SchedulerWrapper<'_>,
    ) -> Result<bool, PanicError> {
        let record = {
            let entry_guard = self.records[txn_idx as usize].lock();
            let entry = entry_guard.as_ref().ok_or_else(|| {
                code_invariant_error(format!(
                    "Recorded output not found when publishing modules for txn {}",
                    txn_idx
                ))
            })?;
            Arc::clone(entry.check_success_or_skip_status()?)
        };

        let mut published = false;
        let mut module_ids_for_v2 = BTreeSet::new();
        record.for_each_module_write(&mut |module_id, state_value| {
            published = true;
            if scheduler.is_v2() {
                module_ids_for_v2.insert(module_id.clone());
            }
            add_module_write_to_module_cache(
                module_id,
                state_value,
                txn_idx,
                runtime_environment,
                global_module_cache,
                versioned_cache.module_cache(),
            )
        })?;
        if published {
            // Record validation requirements after the modules are published.
            scheduler.record_validation_requirements(txn_idx, module_ids_for_v2)?;
        }
        Ok(published)
    }

    pub(crate) fn delayed_field_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = DelayedFieldID>> {
        let record = self.record_if_success_or_skip_rest(txn_idx)?;
        Some(
            record
                .delayed_field_change_set()
                .expect("Output must be set")
                .into_keys(),
        )
    }

    // Called when a transaction is committed to materialize its stored record into
    // the committed output: WriteOps for materialized aggregator values corresponding
    // to the (deltas) in the recorded final output of the transaction, finalized
    // group updates, and materialized events.
    pub(crate) fn materialize<S: TStateView<Key = <R::Txn as Transaction>::Key> + Sync>(
        &self,
        txn_idx: TxnIndex,
        args: &ViewArgs<'_, R, S>,
        environment: &AptosEnvironment,
    ) -> Result<R::CommittedOutput, PanicError> {
        let record = self
            .record_if_success_or_skip_rest(txn_idx)
            .ok_or_else(|| {
                // Only committed (success / skip-rest) outputs are materialized, so a
                // non-committed status here is a code invariant violation.
                code_invariant_error(
                    "Only committed (success / skip-rest) outputs can be materialized",
                )
            })?;
        record
            .materialize(args, txn_idx, environment)
            .map_err(|e| match e {
                PanicOr::CodeInvariantError(msg) => PanicError::CodeInvariantError(msg),
                // Parallel materialization converts group serialization failures to
                // invariant errors internally, so this arm is unreachable.
                PanicOr::Or(e) => {
                    code_invariant_error(format!("Panic error in serializing groups {e:?}"))
                },
            })
    }

    // Returns the record only when the entry exists and its status is success
    // or skip-rest (mirrors the statuses under which an output is stored).
    fn record_if_success_or_skip_rest(&self, txn_idx: TxnIndex) -> Option<Arc<R>> {
        let entry_guard = self.records[txn_idx as usize].lock();
        let entry = entry_guard.as_ref()?;
        entry
            .record
            .status()
            .is_success_or_skip_rest()
            .then(|| Arc::clone(&entry.record))
    }
}
