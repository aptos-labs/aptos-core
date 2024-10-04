// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    components::{block_tree::BlockTree, make_chunk_output::MakeChunkOutput},
    logging::{LogEntry, LogSchema},
    metrics::{
        COMMIT_BLOCKS, CONCURRENCY_GAUGE, EXECUTE_BLOCK, OTHER_TIMERS, SAVE_TRANSACTIONS,
        TRANSACTIONS_SAVED, UPDATE_LEDGER, VM_EXECUTE_BLOCK,
    },
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_executor_types::{
    chunk_output::ChunkOutput, execution_output::ExecutionOutput,
    state_checkpoint_output::StateCheckpointOutput, BlockExecutorTrait, ExecutorError,
    ExecutorResult, StateComputeResult,
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
use fail::fail_point;
use std::{marker::PhantomData, sync::Arc};
use crate::components::block_tree::BlockOutput;
use crate::components::make_ledger_update::MakeLedgerUpdate;
use crate::components::make_state_checkpoint::MakeStateCheckpoint;

pub trait TransactionBlockExecutor: Send + Sync {
    fn execute_transaction_block(
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
        append_state_checkpoint_to_block: Option<HashValue>,
    ) -> Result<ChunkOutput>;
}

impl TransactionBlockExecutor for AptosVM {
    fn execute_transaction_block(
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
        append_state_checkpoint_to_block: Option<HashValue>,
    ) -> Result<ChunkOutput> {
        MakeChunkOutput::by_transaction_execution::<AptosVM>(
            transactions,
            state_view,
            onchain_config,
            append_state_checkpoint_to_block,
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

    /* FIXME(aldenhu): remove
    pub fn root_smt(&self) -> SparseMerkleTree<StateValue> {
        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .root_smt()
    }
     */

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

    fn pre_commit_block(
        &self,
        block_id: HashValue,
        parent_block_id: HashValue,
    ) -> ExecutorResult<()> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["block", "pre_commit_block"]);

        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .pre_commit_block(block_id, parent_block_id)
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
    phantom: PhantomData<V>,
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
            phantom: PhantomData,
        })
    }

    /* FIXME(aldenhu): remove?
    fn root_smt(&self) -> SparseMerkleTree<StateValue> {
        self.block_tree.root_block().output.state().current.clone()
    }
     */

    fn committed_block_id(&self) -> HashValue {
        self.block_tree.root_block().id
    }

    fn execute_and_state_checkpoint(
        &self,
        block: ExecutableBlock,
        parent_block_id: HashValue,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> ExecutorResult<()> {
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
        let (chunk_output, state_checkpoint_output) =
            if parent_block_id != committed_block_id && parent_output.ends_epoch() {
                // ignore reconfiguration suffix, even if the block is non-empty
                info!(
                    LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
                    "reconfig_descendant_block_received"
                );
                (
                    parent_output.chunk_output.new_empty_following_this(),
                    parent_output.expect_state_checkpoint_output().new_empty_following_this(),
                )
            } else {
                let state_view = {
                    let _timer = OTHER_TIMERS.timer_with(&["verified_state_view"]);

                    info!("next_version: {}", parent_output.next_version());
                    CachedStateView::new(
                        StateViewId::BlockExecution { block_id },
                        Arc::clone(&self.db.reader),
                        parent_output.chunk_output.next_version(),
                        &parent_output.expect_state_checkpoint_output().result_state.current,
                        Arc::new(AsyncProofFetcher::new(self.db.reader.clone())),
                    )?
                };

                let chunk_output = {
                    let _timer = VM_EXECUTE_BLOCK.start_timer();
                    fail_point!("executor::vm_execute_block", |_| {
                        Err(ExecutorError::from(anyhow::anyhow!(
                            "Injected error in vm_execute_block"
                        )))
                    });
                    V::execute_transaction_block(
                        transactions,
                        state_view,
                        onchain_config.clone(),
                        Some(block_id),
                    )?
                };
                chunk_output.ensure_is_block()?;

                let _timer = OTHER_TIMERS.timer_with(&["state_checkpoint"]);

                THREAD_MANAGER.get_exe_cpu_pool().install(|| {
                    MakeStateCheckpoint::make(
                        &chunk_output,
                        &parent_output.expect_state_checkpoint_output().result_state,
                        None, /* known_state_checkpoints */
                        true, /* is_block */
                    )
                })?
            };

        let output = BlockOutput::new(chunk_output);
        // TODO(aldenhu): move to next stage (maybe new separate stage)
        output.set_state_checkpoint_output_once(state_checkpoint_output);

        let _ = self.block_tree.add_block(
            parent_block_id,
            block_id,
            output,
        )?;
        Ok(())
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
        let block = block_vec.pop().expect("Must exist").unwrap();
        parent_block.ensure_has_child(block_id)?;

        if block.output.has_ledger_update_output() {
            return Ok(block
                .output
                .as_state_compute_result(
                    &parent_block.output.expect_ledger_update_output().transaction_accumulator,
                ));
        }

        let ledger_update_output = THREAD_MANAGER.get_non_exe_cpu_pool().install(|| {
                    MakeLedgerUpdate::make(

                    )
                })?;
                output
            };

        let state_compute_result = output.as_state_compute_result(
            parent_accumulator,
            block.output.epoch_state().clone(),
        );
        block.output.set_ledger_update(output);
        Ok(state_compute_result)
    }

    fn pre_commit_block(
        &self,
        block_id: HashValue,
        parent_block_id: HashValue,
    ) -> ExecutorResult<()> {
        let _timer = COMMIT_BLOCKS.start_timer();
        info!(
            LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
            "pre_commit_block",
        );

        let mut blocks = self.block_tree.get_blocks(&[parent_block_id, block_id])?;
        let block = blocks.pop().expect("guaranteed");
        let parent_block = blocks.pop().expect("guaranteed");

        let result_in_memory_state = block.output.state().clone();

        fail_point!("executor::pre_commit_block", |_| {
            Err(anyhow::anyhow!("Injected error in pre_commit_block.").into())
        });

        let ledger_update = block.output.get_ledger_update();
        if !ledger_update.transactions_to_commit().is_empty() {
            let _timer = SAVE_TRANSACTIONS.start_timer();
            self.db.writer.pre_commit_ledger(
                ledger_update.transactions_to_commit(),
                ledger_update.first_version(),
                parent_block.output.state().base_version,
                false,
                result_in_memory_state,
                // TODO(grao): Avoid this clone.
                ledger_update.state_updates_until_last_checkpoint.clone(),
                Some(&ledger_update.sharded_state_cache),
            )?;
            TRANSACTIONS_SAVED.observe(ledger_update.num_txns() as f64);
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
