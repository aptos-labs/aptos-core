// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    components::{block_tree::BlockTree, chunk_output::ChunkOutput},
    logging::{LogEntry, LogSchema},
    metrics::{
        APTOS_EXECUTOR_COMMIT_BLOCKS_SECONDS, APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS,
        APTOS_EXECUTOR_OTHER_TIMERS_SECONDS, APTOS_EXECUTOR_SAVE_TRANSACTIONS_SECONDS,
        APTOS_EXECUTOR_TRANSACTIONS_SAVED, APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS,
    },
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_executor_types::{BlockExecutorTrait, Error, StateComputeResult};
use aptos_infallible::RwLock;
use aptos_logger::prelude::*;
use aptos_scratchpad::SparseMerkleTree;
use aptos_state_view::StateViewId;
use aptos_storage_interface::{
    async_proof_fetcher::AsyncProofFetcher, cached_state_view::CachedStateView, DbReaderWriter,
};
use aptos_types::{
    block_executor::partitioner::{ExecutableBlock, ExecutableTransactions},
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValue,
};
use aptos_vm::AptosVM;
use fail::fail_point;
use std::{marker::PhantomData, sync::Arc};

pub trait TransactionBlockExecutor: Send + Sync {
    fn execute_transaction_block(
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<ChunkOutput>;
}

impl TransactionBlockExecutor for AptosVM {
    fn execute_transaction_block(
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<ChunkOutput> {
        ChunkOutput::by_transaction_execution::<AptosVM>(
            transactions,
            state_view,
            maybe_block_gas_limit,
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

    fn execute_block(
        &self,
        block: ExecutableBlock,
        parent_block_id: HashValue,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<StateComputeResult, Error> {
        self.maybe_initialize()?;
        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .execute_block(block, parent_block_id, maybe_block_gas_limit)
    }

    fn commit_blocks_ext(
        &self,
        block_ids: Vec<HashValue>,
        ledger_info_with_sigs: LedgerInfoWithSignatures,
        save_state_snapshots: bool,
    ) -> Result<(), Error> {
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
        self.block_tree
            .root_block()
            .output
            .result_view
            .state()
            .current
            .clone()
    }
}

impl<V> BlockExecutorInner<V>
where
    V: TransactionBlockExecutor,
{
    fn committed_block_id(&self) -> HashValue {
        self.block_tree.root_block().id
    }

    fn execute_block(
        &self,
        block: ExecutableBlock,
        parent_block_id: HashValue,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<StateComputeResult, Error> {
        let _timer = APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS.start_timer();
        let ExecutableBlock {
            block_id,
            transactions,
        } = block;
        let committed_block_id = self.committed_block_id();
        let mut block_vec = self
            .block_tree
            .get_blocks_opt(&[block_id, parent_block_id])?;
        let parent_block = block_vec
            .pop()
            .expect("Must exist.")
            .ok_or(Error::BlockNotFound(parent_block_id))?;
        let parent_output = &parent_block.output;
        let parent_view = &parent_output.result_view;
        let parent_accumulator = parent_view.txn_accumulator();

        if let Some(b) = block_vec.pop().expect("Must exist") {
            // this is a retry
            parent_block.ensure_has_child(block_id)?;
            return Ok(b.output.as_state_compute_result(parent_accumulator));
        }

        let output = if parent_block_id != committed_block_id && parent_output.has_reconfiguration()
        {
            info!(
                LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
                "reconfig_descendant_block_received"
            );
            parent_output.reconfig_suffix()
        } else {
            info!(
                LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
                "execute_block"
            );
            let state_view = {
                let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                    .with_label_values(&["verified_state_view"])
                    .start_timer();
                parent_view.verified_state_view(
                    StateViewId::BlockExecution { block_id },
                    Arc::clone(&self.db.reader),
                    Arc::new(AsyncProofFetcher::new(self.db.reader.clone())),
                )?
            };

            let chunk_output = {
                let _timer = APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS.start_timer();
                fail_point!("executor::vm_execute_block", |_| {
                    Err(Error::from(anyhow::anyhow!(
                        "Injected error in vm_execute_block"
                    )))
                });
                V::execute_transaction_block(transactions, state_view, maybe_block_gas_limit)?
            };
            chunk_output.trace_log_transaction_status();

            let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                .with_label_values(&["apply_to_ledger"])
                .start_timer();

            let (output, _, _) = chunk_output
                .apply_to_ledger_for_block(parent_view, maybe_block_gas_limit.map(|_| block_id))?;

            output
        };
        output.ensure_ends_with_state_checkpoint()?;

        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["as_state_compute_result"])
            .start_timer();
        let block = self
            .block_tree
            .add_block(parent_block_id, block_id, output)?;
        Ok(block.output.as_state_compute_result(parent_accumulator))
    }

    fn commit_blocks_ext(
        &self,
        block_ids: Vec<HashValue>,
        ledger_info_with_sigs: LedgerInfoWithSignatures,
        sync_commit: bool,
    ) -> Result<(), Error> {
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
            .result_view
            .txn_accumulator()
            .num_leaves();

        let to_commit = blocks
            .iter()
            .map(|block| block.output.to_commit.len())
            .sum();
        let target_version = ledger_info_with_sigs.ledger_info().version();
        if first_version + to_commit as u64 != target_version + 1 {
            return Err(Error::BadNumTxnsToCommit {
                first_version,
                to_commit,
                target_version,
            });
        }
        fail_point!("executor::commit_blocks", |_| {
            Err(anyhow::anyhow!("Injected error in commit_blocks.").into())
        });

        for (i, block) in blocks.iter().enumerate() {
            let txns_to_commit: Vec<_> = {
                let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                    .with_label_values(&["get_txns_to_commit"])
                    .start_timer();
                block.output.transactions_to_commit()
            };

            let _timer = APTOS_EXECUTOR_SAVE_TRANSACTIONS_SECONDS.start_timer();
            APTOS_EXECUTOR_TRANSACTIONS_SAVED.observe(to_commit as f64);

            let result_in_memory_state = block.output.result_view.state().clone();
            self.db.writer.save_transaction_block(
                &txns_to_commit,
                first_version,
                committed_block.output.result_view.state().base_version,
                if i == blocks.len() - 1 {
                    Some(&ledger_info_with_sigs)
                } else {
                    None
                },
                sync_commit,
                result_in_memory_state,
                // TODO(grao): Avoid this clone.
                block.output.block_state_updates.clone(),
                &block.output.sharded_state_cache,
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
