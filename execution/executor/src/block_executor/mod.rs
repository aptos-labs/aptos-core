// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    logging::{LogEntry, LogSchema},
    metrics::{
        BLOCK_EXECUTOR_EXECUTE_BLOCK, BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK, COMMIT_BLOCKS, CONCURRENCY_GAUGE, EXECUTE_BLOCK, OTHER_TIMERS, SAVE_TRANSACTIONS, TRANSACTIONS_SAVED, UPDATE_LEDGER
    },
    types::partial_state_compute_result::PartialStateComputeResult,
    workflow::{
        do_get_execution_output::DoGetExecutionOutput, do_ledger_update::DoLedgerUpdate,
        do_state_checkpoint::DoStateCheckpoint,
    },
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_executor_types::{
    execution_output::ExecutionOutput, state_checkpoint_output::StateCheckpointOutput,
    state_compute_result::StateComputeResult, BlockExecutorTrait, ExecutorError, ExecutorResult,
};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_infallible::RwLock;
use aptos_logger::prelude::*;
use aptos_metrics_core::{IntGaugeHelper, TimerHelper};
use aptos_scratchpad::SparseMerkleTree;
use aptos_storage_interface::{
    async_proof_fetcher::AsyncProofFetcher, cached_state_view::CachedStateView, DbReaderWriter,
};
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain,
        partitioner::{ExecutableBlock, ExecutableTransactions},
    },
    ledger_info::LedgerInfoWithSignatures,
    state_store::{state_value::StateValue, StateViewId},
};
use aptos_vm::AptosVM;
use block_tree::BlockTree;
use fail::fail_point;
use std::{sync::Arc};

pub mod block_tree;

pub trait TransactionBlockExecutor: Send + Sync {
    fn new() -> Self;

    fn execute_transaction_block(
        &self,
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<ExecutionOutput>;
}

pub struct AptosVMBlockExecutor;

impl TransactionBlockExecutor for AptosVMBlockExecutor {
    fn new() -> Self {
        Self
    }

    fn execute_transaction_block(
        &self,
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<ExecutionOutput> {
        let _timer = BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK.start_timer();
        DoGetExecutionOutput::by_transaction_execution::<AptosVM>(
            transactions,
            state_view,
            onchain_config,
        )
    }
}

pub struct BlockExecutor<V> {
    pub db: DbReaderWriter,
    inner: RwLock<Option<BlockExecutorInner<V>>>,
}

impl<V> BlockExecutor<V>
where
    V: TransactionBlockExecutor,
{
    pub fn new(db: DbReaderWriter) -> Self {
        Self {
            db,
            inner: RwLock::new(None),
        }
    }

    pub fn root_smt(&self) -> SparseMerkleTree<StateValue> {
        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .root_smt()
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
    V: TransactionBlockExecutor,
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

    fn execute_and_state_checkpoint(
        &self,
        block: ExecutableBlock,
        parent_block_id: HashValue,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> ExecutorResult<StateCheckpointOutput> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["block", "execute_and_state_checkpoint"]);

        self.maybe_initialize()?;
        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .execute_and_state_checkpoint(block, parent_block_id, onchain_config)
    }

    fn ledger_update(
        &self,
        block_id: HashValue,
        parent_block_id: HashValue,
        state_checkpoint_output: StateCheckpointOutput,
    ) -> ExecutorResult<StateComputeResult> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["block", "ledger_update"]);

        self.maybe_initialize()?;
        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .ledger_update(block_id, parent_block_id, state_checkpoint_output)
    }

    fn pre_commit_block(&self, block_id: HashValue) -> ExecutorResult<()> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["block", "pre_commit_block"]);

        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .pre_commit_block(block_id)
    }

    fn commit_ledger(&self, ledger_info_with_sigs: LedgerInfoWithSignatures) -> ExecutorResult<()> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["block", "commit_ledger"]);

        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .commit_ledger(ledger_info_with_sigs)
    }

    fn finish(&self) {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["block", "finish"]);

        *self.inner.write() = None;
    }
}

struct BlockExecutorInner<V> {
    db: DbReaderWriter,
    block_tree: BlockTree,
    block_executor: V,
}

impl<V> BlockExecutorInner<V>
where
    V: TransactionBlockExecutor,
{
    pub fn new(db: DbReaderWriter) -> Result<Self> {
        let block_tree = BlockTree::new(&db.reader)?;
        Ok(Self {
            db,
            block_tree,
            block_executor: V::new(),
        })
    }

    fn root_smt(&self) -> SparseMerkleTree<StateValue> {
        self.block_tree.root_block().output.state().current.clone()
    }
}

impl<V> BlockExecutorInner<V>
where
    V: TransactionBlockExecutor,
{
    fn committed_block_id(&self) -> HashValue {
        self.block_tree.root_block().id
    }

    fn execute_and_state_checkpoint(
        &self,
        block: ExecutableBlock,
        parent_block_id: HashValue,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> ExecutorResult<StateCheckpointOutput> {
        let _timer = EXECUTE_BLOCK.start_timer();
        let ExecutableBlock {
            block_id,
            transactions,
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
            LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
            "execute_block"
        );
        let committed_block_id = self.committed_block_id();
        let (state, epoch_state, state_checkpoint_output) =
            if parent_block_id != committed_block_id && parent_output.has_reconfiguration() {
                // ignore reconfiguration suffix, even if the block is non-empty
                info!(
                    LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
                    "reconfig_descendant_block_received"
                );
                (
                    parent_output.state().clone(),
                    parent_output.epoch_state().clone(),
                    StateCheckpointOutput::default(),
                )
            } else {
                let state_view = {
                    let _timer = OTHER_TIMERS.timer_with(&["verified_state_view"]);

                    info!("next_version: {}", parent_output.next_version());
                    CachedStateView::new(
                        StateViewId::BlockExecution { block_id },
                        Arc::clone(&self.db.reader),
                        parent_output.next_version(),
                        parent_output.state().current.clone(),
                        Arc::new(AsyncProofFetcher::new(self.db.reader.clone())),
                    )?
                };

                let chunk_output = {
                    let _timer = BLOCK_EXECUTOR_EXECUTE_BLOCK.start_timer();
                    fail_point!("executor::block_executor_execute_block", |_| {
                        Err(ExecutorError::from(anyhow::anyhow!(
                            "Injected error in block_executor_execute_block"
                        )))
                    });
                    self.block_executor.execute_transaction_block(transactions, state_view, onchain_config.clone())?
                };

                let _timer = OTHER_TIMERS.timer_with(&["state_checkpoint"]);

                THREAD_MANAGER.get_exe_cpu_pool().install(|| {
                    fail_point!("executor::block_state_checkpoint", |_| {
                        Err(anyhow::anyhow!("Injected error in block state checkpoint."))
                    });

                    DoStateCheckpoint::run(
                        chunk_output,
                        parent_output.state(),
                        Some(block_id),
                        None,
                        /*is_block=*/ true,
                    )
                })?
            };

        let _ = self.block_tree.add_block(
            parent_block_id,
            block_id,
            PartialStateComputeResult::new(parent_output.result_state.clone(), state, epoch_state),
        )?;
        Ok(state_checkpoint_output)
    }

    fn ledger_update(
        &self,
        block_id: HashValue,
        parent_block_id: HashValue,
        state_checkpoint_output: StateCheckpointOutput,
    ) -> ExecutorResult<StateComputeResult> {
        let _timer = UPDATE_LEDGER.start_timer();
        info!(
            LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
            "ledger_update"
        );
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
        let parent_output = parent_block.output.expect_ledger_update_output();
        let parent_accumulator = parent_output.txn_accumulator();
        let block = block_vec.pop().expect("Must exist").unwrap();
        parent_block.ensure_has_child(block_id)?;
        if let Some(complete_result) = block.output.get_complete_result() {
            return Ok(complete_result);
        }

        let output =
            if parent_block_id != committed_block_id && parent_block.output.has_reconfiguration() {
                info!(
                    LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
                    "reconfig_descendant_block_received"
                );
                parent_output.reconfig_suffix()
            } else {
                let (output, _, _) = THREAD_MANAGER.get_non_exe_cpu_pool().install(|| {
                    DoLedgerUpdate::run(state_checkpoint_output, parent_accumulator.clone())
                })?;
                output
            };

        if !block.output.has_reconfiguration() {
            output.ensure_ends_with_state_checkpoint()?;
        }

        block.output.set_ledger_update_output(output);
        Ok(block.output.expect_complete_result())
    }

    fn pre_commit_block(&self, block_id: HashValue) -> ExecutorResult<()> {
        let _timer = COMMIT_BLOCKS.start_timer();
        info!(
            LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
            "pre_commit_block",
        );

        let block = self.block_tree.get_block(block_id)?;

        fail_point!("executor::pre_commit_block", |_| {
            Err(anyhow::anyhow!("Injected error in pre_commit_block.").into())
        });

        let output = block.output.expect_complete_result();
        let num_txns = output.transactions_to_commit_len();
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

        let block_id = ledger_info_with_sigs.ledger_info().consensus_block_id();
        info!(
            LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
            "commit_ledger"
        );

        // Check for any potential retries
        // TODO: do we still have such retries?
        let committed_block = self.block_tree.root_block();
        if committed_block.num_persisted_transactions()
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
}
