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
use storage_interface::{async_proof_fetcher::AsyncProofFetcher, proof_fetcher::ProofFetcher};

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
    inner: RwLock<Option<Arc<BlockExecutorInner<V>>>>,
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
        self.get_inner().root_smt()
    }

    fn get_inner(&self) -> Arc<BlockExecutorInner<V>> {
        self.inner
            .read()
            .clone()
            .expect("BlockExecutor is not reset!")
    }

    fn maybe_initialize(&self) {
        warn!("Start maybe_initialize()");
        let empty_inner = self.inner.read().is_none();
        if empty_inner {
            self.reset();
        }
        warn!("End maybe_initialize()");
    }
}

impl<V> BlockExecutorTrait for BlockExecutor<V>
where
    V: VMExecutor,
{
    fn committed_block_id(&self) -> HashValue {
        warn!("Start committed_block_id()");
        self.maybe_initialize();
        let result = self.get_inner().committed_block_id();
        warn!("End committed_block_id()");
        result
    }

    fn reset(&self) {
        warn!("Start reset()");
        *self.inner.write() = Some(Arc::new(BlockExecutorInner::new(self.db.clone())));
        warn!("End reset()");
    }

    fn execute_block(
        &self,
        block: (HashValue, Vec<Transaction>),
        parent_block_id: HashValue,
    ) -> Result<StateComputeResult, Error> {
        warn!("Start execute_block()");
        self.maybe_initialize();
        let inner = self.get_inner();
        warn!("Got inner execute_block()!");
        let result = inner.execute_block(block, parent_block_id);
        warn!("End execute_block()");
        result
    }

    fn commit_blocks_ext(
        &self,
        block_ids: Vec<HashValue>,
        ledger_info_with_sigs: LedgerInfoWithSignatures,
        save_state_snapshots: bool,
    ) -> Result<(), Error> {
        warn!("Start commit_blocks_ext()");
        let inner = self.get_inner();
        warn!("Got inner commit_blocks_ext()!");
        let result =
            inner.commit_blocks_ext(block_ids, ledger_info_with_sigs, save_state_snapshots);
        warn!("End commit_blocks_ext()");
        result
    }

    fn finish(&self) {
        *self.inner.write() = None;
    }
}

struct BlockExecutorInner<V> {
    db: DbReaderWriter,
    block_tree: BlockTree,
    proof_fetcher: Arc<dyn ProofFetcher>,
    phantom: PhantomData<V>,
}

impl<V> BlockExecutorInner<V>
where
    V: VMExecutor,
{
    pub fn new(db: DbReaderWriter) -> Self {
        let block_tree = BlockTree::new(&db.reader).expect("Block tree failed to init.");
        let proof_fetcher = Arc::new(AsyncProofFetcher::new(db.reader.clone()));
        Self {
            db,
            block_tree,
            proof_fetcher,
            phantom: PhantomData,
        }
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
        warn!("1 execute_block()");
        let (block_id, transactions) = block;
        let committed_block = self.block_tree.root_block();
        warn!("2 execute_block()");
        let mut block_vec = self
            .block_tree
            .get_blocks_opt(&[block_id, parent_block_id])?;
        warn!("3 execute_block()");
        let parent_block = block_vec
            .pop()
            .expect("Must exist.")
            .ok_or(Error::BlockNotFound(parent_block_id))?;
        let parent_output = &parent_block.output;
        let parent_view = &parent_output.result_view;
        let parent_accumulator = parent_view.txn_accumulator();
        warn!("4 execute_block()");

        if let Some(b) = block_vec.pop().expect("Must exist") {
            warn!("5a execute_block()");
            // this is a retry
            return Ok(b.output.as_state_compute_result(parent_accumulator));
        }
        warn!("5b execute_block()");

        let output = if parent_block_id != committed_block.id && parent_output.has_reconfiguration()
        {
            warn!("6a execute_block()");
            info!(
                LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
                "reconfig_descendant_block_received"
            );
            parent_output.reconfig_suffix()
        } else {
            warn!("6b execute_block()");
            info!(
                LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
                "execute_block"
            );
            let _timer = APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS.start_timer();
            let state_view = parent_view.verified_state_view(
                StateViewId::BlockExecution { block_id },
                Arc::clone(&self.db.reader),
                Arc::clone(&self.proof_fetcher),
            )?;
            warn!("6c execute_block()");

            let chunk_output = {
                let _timer = APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS.start_timer();
                fail_point!("executor::vm_execute_block", |_| {
                    Err(Error::from(anyhow::anyhow!(
                        "Injected error in vm_execute_block"
                    )))
                });
                ChunkOutput::by_transaction_execution::<V>(transactions, state_view)?
            };
            warn!("6d execute_block()");
            chunk_output.trace_log_transaction_status();
            warn!("6e execute_block()");

            let (output, _, _) = chunk_output.apply_to_ledger(parent_view)?;
            output
        };
        warn!("7 execute_block()");
        output.ensure_ends_with_state_checkpoint()?;

        let block = self
            .block_tree
            .add_block(parent_block_id, block_id, output)?;
        warn!("8 execute_block()");
        Ok(block.output.as_state_compute_result(parent_accumulator))
    }

    fn commit_blocks_ext(
        &self,
        block_ids: Vec<HashValue>,
        ledger_info_with_sigs: LedgerInfoWithSignatures,
        sync_commit: bool,
    ) -> Result<(), Error> {
        warn!("1 commit_blocks_ext()");
        let _timer = APTOS_EXECUTOR_COMMIT_BLOCKS_SECONDS.start_timer();
        let committed_block = self.block_tree.root_block();
        warn!("2 commit_blocks_ext()");
        if committed_block.num_persisted_transactions()
            == ledger_info_with_sigs.ledger_info().version() + 1
        {
            // a retry
            return Ok(());
        }
        warn!("3 commit_blocks_ext()");

        let block_id_to_commit = ledger_info_with_sigs.ledger_info().consensus_block_id();
        info!(
            LogSchema::new(LogEntry::BlockExecutor).block_id(block_id_to_commit),
            "commit_block"
        );
        warn!("4 commit_blocks_ext()");

        let blocks = self.block_tree.get_blocks(&block_ids)?;
        warn!("5 commit_blocks_ext()");
        let txns_to_commit: Vec<_> = blocks
            .into_iter()
            .map(|block| block.output.transactions_to_commit())
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect();
        warn!("6 commit_blocks_ext()");
        let first_version = committed_block
            .output
            .result_view
            .txn_accumulator()
            .num_leaves();
        warn!("7 commit_blocks_ext()");
        let to_commit = txns_to_commit.len();
        let target_version = ledger_info_with_sigs.ledger_info().version();
        if first_version + txns_to_commit.len() as u64 != target_version + 1 {
            return Err(Error::BadNumTxnsToCommit {
                first_version,
                to_commit,
                target_version,
            });
        }
        warn!("8 commit_blocks_ext()");

        let _timer = APTOS_EXECUTOR_SAVE_TRANSACTIONS_SECONDS.start_timer();
        APTOS_EXECUTOR_TRANSACTIONS_SAVED.observe(to_commit as f64);
        warn!("9 commit_blocks_ext()");

        fail_point!("executor::commit_blocks", |_| {
            Err(anyhow::anyhow!("Injected error in commit_blocks.").into())
        });
        warn!("10 commit_blocks_ext()");
        let result_in_memory_state = self
            .block_tree
            .get_block(block_id_to_commit)?
            .output
            .result_view
            .state()
            .clone();
        warn!("11 commit_blocks_ext()");
        self.db.writer.save_transactions(
            &txns_to_commit,
            first_version,
            committed_block.output.result_view.state().base_version,
            Some(&ledger_info_with_sigs),
            sync_commit,
            result_in_memory_state,
        )?;
        warn!("12 commit_blocks_ext()");
        self.block_tree
            .prune(ledger_info_with_sigs.ledger_info())
            .expect("Failure pruning block tree.");
        warn!("13 commit_blocks_ext()");

        Ok(())
    }
}
