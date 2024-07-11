// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    components::{
        apply_chunk_output::ApplyChunkOutput, block_tree::BlockTree, chunk_output::ChunkOutput,
    },
    logging::{LogEntry, LogSchema},
    metrics::{
        APTOS_EXECUTOR_COMMIT_BLOCKS_SECONDS, APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS,
        APTOS_EXECUTOR_LEDGER_UPDATE_SECONDS, APTOS_EXECUTOR_OTHER_TIMERS_SECONDS,
        APTOS_EXECUTOR_SAVE_TRANSACTIONS_SECONDS, APTOS_EXECUTOR_TRANSACTIONS_SAVED,
        APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS,
    },
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_executor_types::{
    execution_output::ExecutionOutput, state_checkpoint_output::StateCheckpointOutput,
    BlockExecutorTrait, ExecutorError, ExecutorResult, StateComputeResult,
};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_infallible::RwLock;
use aptos_logger::prelude::*;
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

pub trait TransactionBlockExecutor: Send + Sync {
    fn execute_transaction_block(
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<ChunkOutput>;
}

impl TransactionBlockExecutor for AptosVM {
    fn execute_transaction_block(
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<ChunkOutput> {
        ChunkOutput::by_transaction_execution::<AptosVM>(transactions, state_view, onchain_config)
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
        self.maybe_initialize().expect("Failed to initialize.");
        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .committed_block_id()
    }

    fn reset(&self) -> Result<()> {
        *self.inner.write() = Some(BlockExecutorInner::new(self.db.clone())?);
        Ok(())
    }

    fn execute_and_state_checkpoint(
        &self,
        block: ExecutableBlock,
        parent_block_id: HashValue,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> ExecutorResult<StateCheckpointOutput> {
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
        self.maybe_initialize()?;
        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .ledger_update(block_id, parent_block_id, state_checkpoint_output)
    }

    fn commit_blocks_ext(
        &self,
        block_ids: Vec<HashValue>,
        ledger_info_with_sigs: LedgerInfoWithSignatures,
        save_state_snapshots: bool,
    ) -> ExecutorResult<()> {
        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .commit_blocks_ext(block_ids, ledger_info_with_sigs, save_state_snapshots)
    }

    fn finish(&self) {
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
        let _timer = APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS.start_timer();
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
                    let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                        .with_label_values(&["verified_state_view"])
                        .start_timer();
                    info!("next_version: {}", parent_output.next_version());
                    CachedStateView::new(
                        StateViewId::Miscellaneous,
                        Arc::clone(&self.db.reader),
                        parent_output.next_version(),
                        parent_output.state().current.clone(),
                        Arc::new(AsyncProofFetcher::new(self.db.reader.clone())),
                    )?
                };

                let chunk_output = {
                    let _timer = APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS.start_timer();
                    fail_point!("executor::vm_execute_block", |_| {
                        Err(ExecutorError::from(anyhow::anyhow!(
                            "Injected error in vm_execute_block"
                        )))
                    });
                    V::execute_transaction_block(transactions, state_view, onchain_config.clone())?
                };

                let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                    .with_label_values(&["state_checkpoint"])
                    .start_timer();

                THREAD_MANAGER.get_exe_cpu_pool().install(|| {
                    chunk_output.into_state_checkpoint_output(parent_output.state(), block_id)
                })?
            };

        let _ = self.block_tree.add_block(
            parent_block_id,
            block_id,
            ExecutionOutput::new(state, epoch_state),
        )?;
        Ok(state_checkpoint_output)
    }

    fn ledger_update(
        &self,
        block_id: HashValue,
        parent_block_id: HashValue,
        state_checkpoint_output: StateCheckpointOutput,
    ) -> ExecutorResult<StateComputeResult> {
        let _timer = APTOS_EXECUTOR_LEDGER_UPDATE_SECONDS.start_timer();
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
        let parent_output = parent_block.output.get_ledger_update();
        let parent_accumulator = parent_output.txn_accumulator();
        let current_output = block_vec.pop().expect("Must exist").unwrap();
        parent_block.ensure_has_child(block_id)?;
        if current_output.output.has_ledger_update() {
            return Ok(current_output
                .output
                .get_ledger_update()
                .as_state_compute_result(
                    parent_accumulator,
                    current_output.output.epoch_state().clone(),
                ));
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
                    ApplyChunkOutput::calculate_ledger_update(
                        state_checkpoint_output,
                        parent_accumulator.clone(),
                    )
                })?;
                output
            };

        if !current_output.output.has_reconfiguration() {
            output.ensure_ends_with_state_checkpoint()?;
        }

        let state_compute_result = output.as_state_compute_result(
            parent_accumulator,
            current_output.output.epoch_state().clone(),
        );
        current_output.output.set_ledger_update(output);
        Ok(state_compute_result)
    }

    fn commit_blocks_ext(
        &self,
        block_ids: Vec<HashValue>,
        ledger_info_with_sigs: LedgerInfoWithSignatures,
        sync_commit: bool,
    ) -> ExecutorResult<()> {
        let _timer = APTOS_EXECUTOR_COMMIT_BLOCKS_SECONDS.start_timer();

        // Ensure the block ids are not empty
        if block_ids.is_empty() {
            return Err(anyhow::anyhow!("Cannot commit 0 blocks!").into());
        }

        // Check for any potential retries
        let mut committed_block = self.block_tree.root_block();
        if committed_block.num_persisted_transactions()
            == ledger_info_with_sigs.ledger_info().version() + 1
        {
            return Ok(());
        }

        // Ensure the last block id matches the ledger info block id to commit
        let block_id_to_commit = ledger_info_with_sigs.ledger_info().consensus_block_id();
        info!(
            LogSchema::new(LogEntry::BlockExecutor).block_id(block_id_to_commit),
            "commit_block"
        );
        let last_block_id = *block_ids.last().unwrap();
        if last_block_id != block_id_to_commit {
            // This should not happen. If it does, we need to panic!
            panic!(
                "Block id to commit ({:?}) does not match last block id ({:?})!",
                block_id_to_commit, last_block_id
            );
        }

        let blocks = self.block_tree.get_blocks(&block_ids)?;

        let mut first_version = committed_block
            .output
            .get_ledger_update()
            .txn_accumulator()
            .num_leaves();

        let to_commit = blocks
            .iter()
            .map(|block| block.output.get_ledger_update().to_commit.len())
            .sum();
        let target_version = ledger_info_with_sigs.ledger_info().version();
        if first_version + to_commit as u64 != target_version + 1 {
            return Err(ExecutorError::BadNumTxnsToCommit {
                first_version,
                to_commit,
                target_version,
            });
        }
        fail_point!("executor::commit_blocks", |_| {
            Err(anyhow::anyhow!("Injected error in commit_blocks.").into())
        });

        for (i, block) in blocks.iter().enumerate() {
            let txns_to_commit = block.output.get_ledger_update().transactions_to_commit();

            let _timer = APTOS_EXECUTOR_SAVE_TRANSACTIONS_SECONDS.start_timer();
            APTOS_EXECUTOR_TRANSACTIONS_SAVED.observe(to_commit as f64);

            let result_in_memory_state = block.output.state().clone();
            self.db.writer.save_transactions(
                txns_to_commit,
                first_version,
                committed_block.output.state().base_version,
                if i == blocks.len() - 1 {
                    Some(&ledger_info_with_sigs)
                } else {
                    None
                },
                sync_commit,
                result_in_memory_state,
                // TODO(grao): Avoid this clone.
                block
                    .output
                    .get_ledger_update()
                    .state_updates_until_last_checkpoint
                    .clone(),
                Some(&block.output.get_ledger_update().sharded_state_cache),
            )?;
            first_version += txns_to_commit.len() as u64;
            committed_block = block.clone();
        }
        self.block_tree
            .prune(ledger_info_with_sigs.ledger_info())
            .expect("Failure pruning block tree.");

        Ok(())
    }
}
