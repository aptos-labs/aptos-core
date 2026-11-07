// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    logging::{LogEntry, LogSchema},
    metrics::{
        BLOCK_EXECUTION_WORKFLOW_WHOLE, COMMIT_BLOCKS, CONCURRENCY_GAUGE,
        GET_BLOCK_EXECUTION_OUTPUT_BY_EXECUTING, OTHER_TIMERS, SAVE_TRANSACTIONS,
        TRANSACTIONS_SAVED, UPDATE_LEDGER,
    },
    types::partial_state_compute_result::PartialStateComputeResult,
    workflow::{
        do_get_execution_output::{DoGetExecutionOutput, RawBlockExecution},
        do_ledger_update::DoLedgerUpdate,
        do_state_checkpoint::DoStateCheckpoint,
    },
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_executor_types::{
    execution_output::ExecutionOutput, state_compute_result::StateComputeResult,
    BlockExecutorTrait, ExecutorError, ExecutorResult,
};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_infallible::{Mutex, RwLock};
use aptos_logger::prelude::*;
use aptos_metrics_core::{IntGaugeVecHelper, TimerHelper};
use aptos_storage_interface::{
    state_store::{
        collateral::{CollateralParent, ShardedCollateralLayers},
        orders::{OrderParent, ShardedOrderLayers},
        positions::{PositionParent, ShardedPositionLayers},
        state_summary::{
            ProvableCollateralStateSummary, ProvablePositionStateSummary, ProvableStateSummary,
        },
        state_view::cached_state_view::{CachedStateView, PendingWrites},
    },
    DbReaderWriter,
};
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain,
        partitioner::{ExecutableBlock, ExecutableTransactions},
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    ledger_info::LedgerInfoWithSignatures,
    state_store::{StateViewId, TStateView},
    transaction::Version,
};
use aptos_vm::VMBlockExecutor;
use block_tree::BlockTree;
use fail::fail_point;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use std::sync::Arc;

pub mod block_tree;

pub struct BlockExecutor<V> {
    pub db: DbReaderWriter,
    inner: RwLock<Option<BlockExecutorInner<V>>>,
    execution_lock: Mutex<()>,
}

impl<V> BlockExecutor<V>
where
    V: VMBlockExecutor,
{
    pub fn new(db: DbReaderWriter) -> Self {
        Self {
            db,
            inner: RwLock::new(None),
            execution_lock: Mutex::new(()),
        }
    }

    fn maybe_initialize(&self) -> Result<()> {
        if self.inner.read().is_none() {
            self.reset()?;
        }
        Ok(())
    }
}

impl<V> BlockExecutorTrait for BlockExecutor<V>
where
    V: VMBlockExecutor,
{
    fn committed_block_id(&self) -> HashValue {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["block", "committed_block_id"]);

        self.maybe_initialize().expect("Failed to initialize.");
        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .committed_block_id()
    }

    fn reset(&self) -> Result<()> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["block", "reset"]);

        *self.inner.write() = Some(BlockExecutorInner::new(self.db.clone())?);
        Ok(())
    }

    fn execute_and_update_state(
        &self,
        block: ExecutableBlock,
        parent_block_id: HashValue,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> ExecutorResult<()> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["block", "execute_and_state_checkpoint"]);

        self.maybe_initialize()?;
        // guarantee only one block being executed at a time
        let _guard = self.execution_lock.lock();
        self.inner
            .read()
            .as_ref()
            .ok_or_else(|| ExecutorError::InternalError {
                error: "BlockExecutor is not reset".into(),
            })?
            .execute_and_update_state(block, parent_block_id, onchain_config)
    }

    fn ledger_update(
        &self,
        block_id: HashValue,
        parent_block_id: HashValue,
    ) -> ExecutorResult<StateComputeResult> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["block", "ledger_update"]);

        self.inner
            .read()
            .as_ref()
            .ok_or_else(|| ExecutorError::InternalError {
                error: "BlockExecutor is not reset".into(),
            })?
            .ledger_update(block_id, parent_block_id)
    }

    fn pre_commit_block(&self, block_id: HashValue) -> ExecutorResult<()> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["block", "pre_commit_block"]);

        self.inner
            .read()
            .as_ref()
            .ok_or_else(|| ExecutorError::InternalError {
                error: "BlockExecutor is not reset".into(),
            })?
            .pre_commit_block(block_id)
    }

    fn commit_ledger(&self, ledger_info_with_sigs: LedgerInfoWithSignatures) -> ExecutorResult<()> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["block", "commit_ledger"]);

        self.inner
            .read()
            .as_ref()
            .ok_or_else(|| ExecutorError::InternalError {
                error: "BlockExecutor is not reset".into(),
            })?
            .commit_ledger(ledger_info_with_sigs)
    }

    fn finish(&self) {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["block", "finish"]);

        *self.inner.write() = None;
    }

    fn state_view(&self, block_id: HashValue) -> ExecutorResult<CachedStateView> {
        self.maybe_initialize()?;
        self.inner.read().as_ref().unwrap().state_view(block_id)
    }
}

/// Experiment knob: overlap a block's output indexing with the next block's
/// execution. `APTOS_PIPELINED_EXECUTION=1` turns it on.
static PIPELINED_EXECUTION: Lazy<bool> =
    Lazy::new(|| std::env::var_os("APTOS_PIPELINED_EXECUTION").is_some());

/// A block the VM has executed whose outputs are still being indexed on
/// another thread. The next block reads its writes through the view until
/// the indexed state is in the block tree.
struct PendingBlock {
    block_id: HashValue,
    parent_block_id: HashValue,
    writes: Arc<PendingWrites>,
    parse: std::thread::JoinHandle<ExecutorResult<ExecutionOutput>>,
}

struct BlockExecutorInner<V> {
    db: DbReaderWriter,
    block_tree: BlockTree,
    block_executor: V,
    pending: Mutex<Option<PendingBlock>>,
}

impl<V> BlockExecutorInner<V>
where
    V: VMBlockExecutor,
{
    pub fn new(db: DbReaderWriter) -> Result<Self> {
        let block_tree = BlockTree::new(&db.reader)?;
        Ok(Self {
            db,
            block_tree,
            block_executor: V::new(),
            pending: Mutex::new(None),
        })
    }

    /// Waits for the pending block's indexing and puts it in the block tree.
    fn finish_pending(&self, pending: &mut Option<PendingBlock>) -> ExecutorResult<()> {
        let Some(block) = pending.take() else {
            return Ok(());
        };
        let _timer = OTHER_TIMERS.timer_with(&["pipelined_wait_for_parse"]);
        let output = block
            .parse
            .join()
            .map_err(|_| ExecutorError::InternalError {
                error: "block indexing thread panicked".into(),
            })??;
        self.block_tree.add_block(
            block.parent_block_id,
            block.block_id,
            PartialStateComputeResult::new(output),
        )?;
        Ok(())
    }

    /// Every read of the block tree from outside execution goes through here,
    /// so a pending block is never missing from it.
    fn flush_pending(&self) -> ExecutorResult<()> {
        let mut pending = self.pending.lock();
        self.finish_pending(&mut pending)
    }

    /// Executes `block` against its parent's writes while the parent's outputs
    /// are still being indexed, then hands this block's outputs to the
    /// indexing thread. Falls back to the serial path for anything it does
    /// not handle: sharded blocks, reconfigurations, trading-native overlays.
    fn execute_pipelined(
        &self,
        block: ExecutableBlock,
        parent_block_id: HashValue,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> ExecutorResult<Option<ExecutableBlock>> {
        let mut pending = self.pending.lock();
        let overlays = match pending.as_ref() {
            Some(p) if p.block_id == parent_block_id => vec![Arc::clone(&p.writes)],
            _ => {
                self.finish_pending(&mut pending)?;
                vec![]
            },
        };
        let base_block_id = match pending.as_ref() {
            Some(p) => p.parent_block_id,
            None => parent_block_id,
        };
        let base_block = self
            .block_tree
            .get_blocks_opt(&[base_block_id])?
            .pop()
            .expect("Must exist.")
            .ok_or(ExecutorError::BlockNotFound(base_block_id))?;
        let base_output = &base_block.output;
        let ExecutableTransactions::Unsharded(transactions) = block.transactions else {
            self.finish_pending(&mut pending)?;
            return Ok(Some(block));
        };
        if base_output.has_reconfiguration()
            || base_output.result_positions().is_some()
            || base_output.result_orders().is_some()
            || base_output.result_collateral().is_some()
        {
            self.finish_pending(&mut pending)?;
            return Ok(Some(ExecutableBlock {
                block_id: block.block_id,
                transactions: ExecutableTransactions::Unsharded(transactions),
                auxiliary_info: block.auxiliary_info,
            }));
        }
        let block_id = block.block_id;
        info!(
            block_id = block_id,
            first_version = overlays
                .last()
                .map_or(base_output.execution_output.next_version(), |w| w
                    .next_version()),
            "execute_block (pipelined)"
        );

        let state_view = {
            let _timer = OTHER_TIMERS.timer_with(&["get_state_view"]);
            CachedStateView::new_pending(
                StateViewId::BlockExecution { block_id },
                Arc::clone(&self.db.reader),
                base_output.result_state().latest().clone(),
                overlays,
            )?
        };
        let raw = {
            let _timer = GET_BLOCK_EXECUTION_OUTPUT_BY_EXECUTING.start_timer();
            DoGetExecutionOutput::execute_raw(
                &self.block_executor,
                transactions,
                block.auxiliary_info,
                state_view,
                onchain_config,
                TransactionSliceMetadata::block(parent_block_id, block_id),
            )?
        };
        let writes = {
            let _timer = OTHER_TIMERS.timer_with(&["pipelined_pending_writes"]);
            Self::pending_writes(&raw)?
        };

        // This block's indexing chains on the parent's result, so the parent
        // has to be in the tree first.
        self.finish_pending(&mut pending)?;
        let parent_state = self
            .block_tree
            .get_block(parent_block_id)?
            .output
            .result_state()
            .clone();
        if raw.has_reconfiguration() {
            let output = DoGetExecutionOutput::parse_raw(raw, &parent_state, None, None, None)?;
            DoGetExecutionOutput::update_counters_async(&output);
            self.block_tree.add_block(
                parent_block_id,
                block_id,
                PartialStateComputeResult::new(output),
            )?;
            return Ok(None);
        }
        let parse = std::thread::Builder::new()
            .name("block_parse".to_string())
            .spawn(move || {
                let _timer = OTHER_TIMERS.timer_with(&["pipelined_parse"]);
                let output = DoGetExecutionOutput::parse_raw(raw, &parent_state, None, None, None)?;
                DoGetExecutionOutput::update_counters_async(&output);
                Ok(output)
            })
            .expect("Failed to spawn the block indexing thread");
        *pending = Some(PendingBlock {
            block_id,
            parent_block_id,
            writes: Arc::new(writes),
            parse,
        });
        Ok(None)
    }

    /// The writes the next block will see from `raw` before it is indexed.
    fn pending_writes(raw: &RawBlockExecution) -> Result<PendingWrites> {
        let first_version = raw.state_view.next_version();
        let block_writes = raw.block_writes.as_ref();
        let last_version = raw.next_version().saturating_sub(1);
        let writes = raw
            .transaction_outputs
            .par_iter()
            .enumerate()
            .flat_map_iter(move |(i, output)| {
                let version = first_version + i as Version;
                // Member writes are not state keys: the next block reads the
                // group's blob, which enters at the block's last version.
                output
                    .write_set()
                    .write_op_iter()
                    .filter(move |(key, _)| {
                        block_writes.is_none_or(|w| !w.member_keys.contains(key))
                    })
                    .map(move |(key, op)| {
                        (
                            key.clone(),
                            version,
                            op.as_base_op().as_state_value_opt().flatten().cloned(),
                        )
                    })
            })
            .chain(
                block_writes
                    .into_par_iter()
                    .flat_map_iter(|w| w.group_writes.iter())
                    .map(move |(key, op)| {
                        (
                            key.clone(),
                            last_version,
                            op.as_base_op().as_state_value_opt().flatten().cloned(),
                        )
                    }),
            );
        THREAD_MANAGER.get_exe_cpu_pool().install(|| {
            PendingWrites::new(
                writes,
                raw.next_version(),
                raw.state_view.get_usage()?,
                |key| raw.state_view.memorized_reads().get_cloned(key),
            )
        })
    }
}

impl<V> BlockExecutorInner<V>
where
    V: VMBlockExecutor,
{
    fn committed_block_id(&self) -> HashValue {
        self.block_tree.root_block().id
    }

    fn execute_and_update_state(
        &self,
        block: ExecutableBlock,
        parent_block_id: HashValue,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> ExecutorResult<()> {
        let _timer = BLOCK_EXECUTION_WORKFLOW_WHOLE.start_timer();
        let block = if *PIPELINED_EXECUTION {
            match self.execute_pipelined(block, parent_block_id, onchain_config)? {
                None => return Ok(()),
                Some(block) => block,
            }
        } else {
            self.flush_pending()?;
            block
        };
        let ExecutableBlock {
            block_id,
            transactions,
            auxiliary_info,
        } = block;
        let mut block_vec = self
            .block_tree
            .get_blocks_opt(&[block_id, parent_block_id])?;
        let parent_block = block_vec
            .pop()
            .expect("Must exist.")
            .ok_or(ExecutorError::BlockNotFound(parent_block_id))?;
        let parent_output = &parent_block.output;
        info!(
            block_id = block_id,
            first_version = parent_output.execution_output.next_version(),
            "execute_block"
        );
        let committed_block_id = self.committed_block_id();
        let position_floor = self.prepare_position_base(block_id, parent_output)?;
        let order_floor = self.prepare_order_base(block_id, parent_output)?;
        let collateral_floor = self.prepare_collateral_base(block_id, parent_output)?;

        // Record ExecutionStart for traced transactions.
        // block_txns is populated at proposal time (round_manager::process_proposal).
        {
            let store = aptos_transaction_tracing::store::TransactionTraceStore::global();
            if store.is_enabled() {
                store.record_block_stage(
                    &block_id,
                    aptos_transaction_tracing::types::TransactionStage::ExecutionStart,
                );
            }
        }

        let execution_output =
            if parent_block_id != committed_block_id && parent_output.has_reconfiguration() {
                // ignore reconfiguration suffix, even if the block is non-empty
                info!(
                    LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
                    "reconfig_descendant_block_received"
                );
                parent_output.execution_output.reconfig_suffix()
            } else {
                let state_view = {
                    let _timer = OTHER_TIMERS.timer_with(&["get_state_view"]);
                    CachedStateView::new(
                        StateViewId::BlockExecution { block_id },
                        Arc::clone(&self.db.reader),
                        parent_output.result_state().latest().clone(),
                    )?
                };

                let _timer = GET_BLOCK_EXECUTION_OUTPUT_BY_EXECUTING.start_timer();
                fail_point!("executor::block_executor_execute_block", |_| {
                    Err(ExecutorError::from(anyhow::anyhow!(
                        "Injected error in block_executor_execute_block"
                    )))
                });

                DoGetExecutionOutput::by_transaction_execution(
                    &self.block_executor,
                    transactions,
                    auxiliary_info,
                    parent_output.result_state(),
                    parent_output
                        .result_positions()
                        .zip(position_floor.as_ref())
                        .map(|(overlay, floor)| PositionParent { overlay, floor }),
                    parent_output
                        .result_orders()
                        .zip(order_floor.as_ref())
                        .map(|(overlay, floor)| OrderParent { overlay, floor }),
                    parent_output
                        .result_collateral()
                        .zip(collateral_floor.as_ref())
                        .map(|(overlay, floor)| CollateralParent { overlay, floor }),
                    state_view,
                    onchain_config,
                    TransactionSliceMetadata::block(parent_block_id, block_id),
                )?
            };

        // Record Executed(Keep/Retry/Discard) for traced txns in this block.
        {
            let store = aptos_transaction_tracing::store::TransactionTraceStore::global();
            if store.is_enabled() {
                let retry_hashes: Vec<HashValue> = execution_output
                    .to_retry
                    .transactions
                    .iter()
                    .map(|t| t.committed_hash())
                    .collect();
                let discard_hashes: Vec<HashValue> = execution_output
                    .to_discard
                    .transactions
                    .iter()
                    .map(|t| t.committed_hash())
                    .collect();
                store.record_execution_result(&block_id, &retry_hashes, &discard_hashes);
            }
        }

        let output = PartialStateComputeResult::new(execution_output);
        let _ = self
            .block_tree
            .add_block(parent_block_id, block_id, output)?;
        Ok(())
    }

    /// Advances the position base, and refuses a block that can no
    /// longer read against it.
    ///
    /// Called under `execution_lock`, so the base can't move while a
    /// block executes — which is what lets one check here cover every
    /// read the VM goes on to make.
    fn prepare_position_base(
        &self,
        block_id: HashValue,
        parent_output: &PartialStateComputeResult,
    ) -> ExecutorResult<Option<ShardedPositionLayers>> {
        let Some(parent_positions) = parent_output.result_positions() else {
            return Ok(None);
        };
        // Advance only to a version this block descends from. A block
        // that doesn't is on a dead branch; leaving the base where it is
        // keeps that block readable until it's pruned.
        let certified = self.block_tree.root_block();
        let target = certified
            .output
            .result_positions()
            .filter(|certified_positions| parent_positions.is_descendant_of(certified_positions));
        let floor = self
            .db
            .writer
            .advance_position_base(target)
            .map_err(anyhow::Error::from)?
            .ok_or_else(|| ExecutorError::InternalError {
                error: "parent carries positions but native-position storage is absent".into(),
            })?;
        if !parent_positions.descends_from(&floor) {
            // The base already sits past this block's branch point, so
            // its reads would resolve against the certified branch.
            return Err(ExecutorError::StaleBranch(block_id));
        }
        Ok(Some(floor))
    }

    /// The order analog of [`Self::prepare_position_base`], under the same
    /// lock.
    fn prepare_order_base(
        &self,
        block_id: HashValue,
        parent_output: &PartialStateComputeResult,
    ) -> ExecutorResult<Option<ShardedOrderLayers>> {
        let Some(parent_orders) = parent_output.result_orders() else {
            return Ok(None);
        };
        let certified = self.block_tree.root_block();
        let target = certified
            .output
            .result_orders()
            .filter(|certified_orders| parent_orders.is_descendant_of(certified_orders));
        let floor = self
            .db
            .writer
            .advance_order_base(target)
            .map_err(anyhow::Error::from)?
            .ok_or_else(|| ExecutorError::InternalError {
                error: "parent carries orders but trading-native storage is absent".into(),
            })?;
        if !parent_orders.descends_from(&floor) {
            return Err(ExecutorError::StaleBranch(block_id));
        }
        Ok(Some(floor))
    }

    /// The collateral analog of [`Self::prepare_position_base`], under the
    /// same lock.
    fn prepare_collateral_base(
        &self,
        block_id: HashValue,
        parent_output: &PartialStateComputeResult,
    ) -> ExecutorResult<Option<ShardedCollateralLayers>> {
        let Some(parent_collateral) = parent_output.result_collateral() else {
            return Ok(None);
        };
        let certified = self.block_tree.root_block();
        let target = certified
            .output
            .result_collateral()
            .filter(|certified_collateral| {
                parent_collateral.is_descendant_of(certified_collateral)
            });
        let floor = self
            .db
            .writer
            .advance_collateral_base(target)
            .map_err(anyhow::Error::from)?
            .ok_or_else(|| ExecutorError::InternalError {
                error: "parent carries collateral but trading-native storage is absent".into(),
            })?;
        if !parent_collateral.descends_from(&floor) {
            return Err(ExecutorError::StaleBranch(block_id));
        }
        Ok(Some(floor))
    }

    fn ledger_update(
        &self,
        block_id: HashValue,
        parent_block_id: HashValue,
    ) -> ExecutorResult<StateComputeResult> {
        let _timer = UPDATE_LEDGER.start_timer();
        info!(
            LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
            "ledger_update"
        );
        self.flush_pending()?;
        let committed_block_id = self.committed_block_id();
        let mut block_vec = self
            .block_tree
            .get_blocks_opt(&[block_id, parent_block_id])?;
        let parent_block = block_vec
            .pop()
            .expect("Must exist.")
            .ok_or(ExecutorError::BlockNotFound(parent_block_id))?;
        // At this point of time two things must happen
        // 1. The block tree must also have the current block id with or without the ledger update output.
        // 2. We must have the ledger update output of the parent block.
        // Above is not ture if the block is on a forked branch.
        let block = block_vec
            .pop()
            .expect("Must exist")
            .ok_or(ExecutorError::BlockNotFound(parent_block_id))?;
        parent_block.ensure_has_child(block_id)?;
        let output = &block.output;
        let parent_out = &parent_block.output;

        // TODO(aldenhu): remove, assuming no retries.
        if let Some(complete_result) = block.output.get_complete_result() {
            info!(block_id = block_id, "ledger_update already done.");
            return Ok(complete_result);
        }

        if parent_block_id != committed_block_id && parent_out.has_reconfiguration() {
            info!(block_id = block_id, "ledger_update for reconfig suffix.");

            // Parent must have done all state checkpoint and ledger update since this method
            // is being called.
            output.set_state_checkpoint_output(
                parent_out
                    .ensure_state_checkpoint_output()?
                    .reconfig_suffix(),
            );
            output.set_ledger_update_output(
                parent_out.ensure_ledger_update_output()?.reconfig_suffix(),
            );
        } else {
            THREAD_MANAGER.get_non_exe_cpu_pool().install(|| {
                // TODO(aldenhu): remove? no known strategy to recover from this failure
                fail_point!("executor::block_state_checkpoint", |_| {
                    Err(anyhow::anyhow!("Injected error in block state checkpoint."))
                });
                let parent_state_summary = parent_block.output.ensure_result_state_summary()?;
                let position_persisted = output
                    .execution_output
                    .compute_trading_native_state_roots
                    .then(|| {
                        ProvablePositionStateSummary::new_persisted_position(
                            self.db.reader.as_ref(),
                        )
                    })
                    .transpose()?;
                let parent_position_summary =
                    parent_block.output.ensure_result_position_state_summary()?;
                let collateral_persisted = output
                    .execution_output
                    .compute_trading_native_state_roots
                    .then(|| {
                        ProvableCollateralStateSummary::new_persisted_collateral(
                            self.db.reader.as_ref(),
                        )
                    })
                    .transpose()?;
                let parent_collateral_summary = parent_block
                    .output
                    .ensure_result_collateral_state_summary()?;
                output.set_state_checkpoint_output(
                    DoStateCheckpoint::run()
                        .execution_output(&output.execution_output)
                        .parent_state_summary(parent_state_summary)
                        .persisted_state_summary(&ProvableStateSummary::new_persisted(
                            self.db.reader.as_ref(),
                        )?)
                        .maybe_parent_position_state_summary(parent_position_summary)
                        .maybe_persisted_position_state_summary(position_persisted.as_ref())
                        .maybe_parent_collateral_state_summary(parent_collateral_summary)
                        .maybe_persisted_collateral_state_summary(collateral_persisted.as_ref())
                        .build()?,
                );
                output.set_ledger_update_output(DoLedgerUpdate::run(
                    &output.execution_output,
                    output.ensure_state_checkpoint_output()?,
                    parent_out
                        .ensure_ledger_update_output()?
                        .transaction_accumulator
                        .clone(),
                )?);
                Result::<_>::Ok(())
            })?;
        }

        Ok(block.output.expect_complete_result())
    }

    fn pre_commit_block(&self, block_id: HashValue) -> ExecutorResult<()> {
        let _timer = COMMIT_BLOCKS.start_timer();
        info!(
            LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
            "pre_commit_block",
        );
        self.flush_pending()?;

        let block = self.block_tree.get_block(block_id)?;

        fail_point!("executor::pre_commit_block", |_| {
            Err(anyhow::anyhow!("Injected error in pre_commit_block.").into())
        });

        let output = block.output.expect_complete_result();
        let num_txns = output.num_transactions_to_commit();
        if num_txns != 0 {
            let _timer = SAVE_TRANSACTIONS.start_timer();
            self.db
                .writer
                .pre_commit_ledger(output.as_chunk_to_commit(), false)?;
            TRANSACTIONS_SAVED.observe(num_txns as f64);
        }

        Ok(())
    }

    fn commit_ledger(&self, ledger_info_with_sigs: LedgerInfoWithSignatures) -> ExecutorResult<()> {
        let _timer = OTHER_TIMERS.timer_with(&["commit_ledger"]);
        self.flush_pending()?;

        let block_id = ledger_info_with_sigs.ledger_info().consensus_block_id();
        info!(
            LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
            "commit_ledger"
        );

        // Check for any potential retries
        // TODO: do we still have such retries?
        let committed_block = self.block_tree.root_block();
        if committed_block.num_persisted_transactions()?
            == ledger_info_with_sigs.ledger_info().version() + 1
        {
            return Ok(());
        }

        // Confirm the block to be committed is tracked in the tree.
        self.block_tree.get_block(block_id)?;

        fail_point!("executor::commit_blocks", |_| {
            Err(anyhow::anyhow!("Injected error in commit_blocks.").into())
        });

        let target_version = ledger_info_with_sigs.ledger_info().version();
        self.db
            .writer
            .commit_ledger(target_version, Some(&ledger_info_with_sigs), None)?;

        self.block_tree.prune(ledger_info_with_sigs.ledger_info())?;

        Ok(())
    }

    fn state_view(&self, block_id: HashValue) -> ExecutorResult<CachedStateView> {
        self.flush_pending()?;
        let mut block_vec = self.block_tree.get_blocks_opt(&[block_id])?;
        let block = block_vec
            .pop()
            .expect("Must exist.")
            .ok_or(ExecutorError::BlockNotFound(block_id))?;
        Ok(CachedStateView::new(
            StateViewId::BlockExecution { block_id },
            Arc::clone(&self.db.reader),
            block.output.result_state().latest().clone(),
        )?)
    }
}
