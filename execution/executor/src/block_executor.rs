// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::logging::{LogEntry, LogSchema};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_infallible::RwLock;
use aptos_logger::prelude::*;
use aptos_state_view::StateViewId;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures, state_store::state_value::StateValue,
    transaction::Transaction,
};
use aptos_vm::VMExecutor;
use executor_types::{BlockExecutorTrait, Error, StateComputeResult};
use fail::fail_point;
use scratchpad::SparseMerkleTree;
use std::{marker::PhantomData, sync::Arc};
use storage_interface::async_proof_fetcher::AsyncProofFetcher;

use crate::{
    components::{block_tree::BlockTree, chunk_output::ChunkOutput},
    metrics::{
        APTOS_EXECUTOR_COMMIT_BLOCKS_SECONDS, APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS,
        APTOS_EXECUTOR_SAVE_TRANSACTIONS_SECONDS, APTOS_EXECUTOR_TRANSACTIONS_SAVED,
        APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS,
    },
};
use storage_interface::DbReaderWriter;

pub struct BlockExecutor<V> {
    pub db: DbReaderWriter,
    inner: RwLock<Option<BlockExecutorInner<V>>>,
}

impl<V> BlockExecutor<V>
where
    V: VMExecutor,
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
    V: VMExecutor,
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
        block: (HashValue, Vec<Transaction>),
        parent_block_id: HashValue,
    ) -> Result<StateComputeResult, Error> {
        self.maybe_initialize()?;
        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .execute_block(block, parent_block_id)
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
    V: VMExecutor,
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
    V: VMExecutor,
{
    fn committed_block_id(&self) -> HashValue {
        self.block_tree.root_block().id
    }

    fn execute_block(
        &self,
        block: (HashValue, Vec<Transaction>),
        parent_block_id: HashValue,
    ) -> Result<StateComputeResult, Error> {
        let (block_id, transactions) = block;
        let committed_block = self.block_tree.root_block();
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

        let output = if parent_block_id != committed_block.id && parent_output.has_reconfiguration()
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
            let _timer = APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS.start_timer();
            let state_view = parent_view.verified_state_view(
                StateViewId::BlockExecution { block_id },
                Arc::clone(&self.db.reader),
                Arc::new(AsyncProofFetcher::new(self.db.reader.clone())),
            )?;

            let chunk_output = {
                let _timer = APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS.start_timer();
                fail_point!("executor::vm_execute_block", |_| {
                    Err(Error::from(anyhow::anyhow!(
                        "Injected error in vm_execute_block"
                    )))
                });
                ChunkOutput::by_transaction_execution::<V>(transactions, state_view)?
            };
            chunk_output.trace_log_transaction_status();

            let (output, _, _) = chunk_output.apply_to_ledger(parent_view)?;
            output
        };
        output.ensure_ends_with_state_checkpoint()?;

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
        let committed_block = self.block_tree.root_block();
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
        let txns_to_commit: Vec<_> = blocks
            .into_iter()
            .map(|block| block.output.transactions_to_commit())
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect();
        let first_version = committed_block
            .output
            .result_view
            .txn_accumulator()
            .num_leaves();
        let to_commit = txns_to_commit.len();
        let target_version = ledger_info_with_sigs.ledger_info().version();
        if first_version + txns_to_commit.len() as u64 != target_version + 1 {
            return Err(Error::BadNumTxnsToCommit {
                first_version,
                to_commit,
                target_version,
            });
        }

        let _timer = APTOS_EXECUTOR_SAVE_TRANSACTIONS_SECONDS.start_timer();
        APTOS_EXECUTOR_TRANSACTIONS_SAVED.observe(to_commit as f64);

        fail_point!("executor::commit_blocks", |_| {
            Err(anyhow::anyhow!("Injected error in commit_blocks.").into())
        });
        let result_in_memory_state = self
            .block_tree
            .get_block(block_id_to_commit)?
            .output
            .result_view
            .state()
            .clone();
        self.db.writer.save_transactions(
            &txns_to_commit,
            first_version,
            committed_block.output.result_view.state().base_version,
            Some(&ledger_info_with_sigs),
            sync_commit,
            result_in_memory_state,
        )?;
        self.block_tree
            .prune(ledger_info_with_sigs.ledger_info())
            .expect("Failure pruning block tree.");

        Ok(())
    }
}
