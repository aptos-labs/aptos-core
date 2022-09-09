// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    components::{
        apply_chunk_output::{ensure_no_discard, ensure_no_retry},
        chunk_commit_queue::ChunkCommitQueue,
        chunk_output::ChunkOutput,
    },
    logging::{LogEntry, LogSchema},
    metrics::{
        APTOS_EXECUTOR_APPLY_CHUNK_SECONDS, APTOS_EXECUTOR_COMMIT_CHUNK_SECONDS,
        APTOS_EXECUTOR_EXECUTE_CHUNK_SECONDS, APTOS_EXECUTOR_VM_EXECUTE_CHUNK_SECONDS,
    },
};
use anyhow::Result;
use aptos_infallible::{Mutex, RwLock};
use aptos_logger::prelude::*;
use aptos_state_view::StateViewId;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    transaction::{
        Transaction, TransactionInfo, TransactionListWithProof, TransactionOutputListWithProof,
    },
};
use aptos_vm::VMExecutor;
use executor_types::{
    ChunkCommitNotification, ChunkExecutorTrait, ExecutedChunk, TransactionReplayer,
};
use fail::fail_point;
use std::{marker::PhantomData, sync::Arc};
use storage_interface::{
    cached_state_view::CachedStateView, sync_proof_fetcher::SyncProofFetcher, DbReaderWriter,
    ExecutedTrees,
};

pub struct ChunkExecutor<V> {
    db: DbReaderWriter,
    inner: RwLock<Option<ChunkExecutorInner<V>>>,
}

impl<V: VMExecutor> ChunkExecutor<V> {
    pub fn new(db: DbReaderWriter) -> Self {
        Self {
            db,
            inner: RwLock::new(None),
        }
    }

    fn maybe_initialize(&self) -> Result<()> {
        if self.inner.read().is_none() {
            self.reset()?;
        }
        Ok(())
    }
}

impl<V: VMExecutor> ChunkExecutorTrait for ChunkExecutor<V> {
    fn execute_chunk(
        &self,
        txn_list_with_proof: TransactionListWithProof,
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()> {
        self.maybe_initialize()?;
        self.inner
            .read()
            .as_ref()
            .expect("not reset")
            .execute_chunk(txn_list_with_proof, verified_target_li, epoch_change_li)
    }

    fn apply_chunk(
        &self,
        txn_output_list_with_proof: TransactionOutputListWithProof,
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()> {
        self.inner.read().as_ref().expect("not reset").apply_chunk(
            txn_output_list_with_proof,
            verified_target_li,
            epoch_change_li,
        )
    }

    fn commit_chunk(&self) -> Result<ChunkCommitNotification> {
        self.inner
            .read()
            .as_ref()
            .expect("not reset")
            .commit_chunk()
    }

    fn reset(&self) -> Result<()> {
        *self.inner.write() = Some(ChunkExecutorInner::new(self.db.clone())?);
        Ok(())
    }

    fn finish(&self) {
        *self.inner.write() = None;
    }
}

struct ChunkExecutorInner<V> {
    db: DbReaderWriter,
    commit_queue: Mutex<ChunkCommitQueue>,
    _phantom: PhantomData<V>,
}

impl<V: VMExecutor> ChunkExecutorInner<V> {
    pub fn new(db: DbReaderWriter) -> Result<Self> {
        let commit_queue = Mutex::new(ChunkCommitQueue::new_from_db(&db.reader)?);
        Ok(Self {
            db,
            commit_queue,
            _phantom: PhantomData,
        })
    }

    fn state_view(&self, latest_view: &ExecutedTrees) -> Result<CachedStateView> {
        latest_view.verified_state_view(
            StateViewId::ChunkExecution {
                first_version: latest_view.txn_accumulator().num_leaves(),
            },
            Arc::clone(&self.db.reader),
            Arc::new(SyncProofFetcher::new(self.db.reader.clone())),
        )
    }

    fn apply_chunk_output_for_state_sync(
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
        latest_view: &ExecutedTrees,
        chunk_output: ChunkOutput,
        transaction_infos: &[TransactionInfo],
    ) -> Result<ExecutedChunk> {
        let (mut executed_chunk, to_discard, to_retry) =
            chunk_output.apply_to_ledger(latest_view)?;
        ensure_no_discard(to_discard)?;
        ensure_no_retry(to_retry)?;
        executed_chunk.ledger_info = executed_chunk
            .maybe_select_chunk_ending_ledger_info(verified_target_li, epoch_change_li)?;
        executed_chunk.ensure_transaction_infos_match(transaction_infos)?;

        Ok(executed_chunk)
    }

    fn commit_chunk_impl(&self) -> Result<Arc<ExecutedChunk>> {
        let (base_view, to_commit) = self.commit_queue.lock().next_chunk_to_commit()?;
        let txns_to_commit = to_commit.transactions_to_commit()?;
        let ledger_info = to_commit.ledger_info.as_ref();
        if ledger_info.is_some() || !txns_to_commit.is_empty() {
            fail_point!("executor::commit_chunk", |_| {
                Err(anyhow::anyhow!("Injected error in commit_chunk"))
            });
            self.db.writer.save_transactions(
                &txns_to_commit,
                base_view.txn_accumulator().num_leaves(),
                base_view.state().base_version,
                ledger_info,
                false, /* sync_commit */
                to_commit.result_view.state().clone(),
            )?;
        }

        self.commit_queue
            .lock()
            .dequeue()
            .expect("commit_queue.deque() failed.");
        Ok(to_commit)
    }

    // ************************* Block Executor Implementation *************************
    fn execute_chunk(
        &self,
        txn_list_with_proof: TransactionListWithProof,
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()> {
        let _timer = APTOS_EXECUTOR_EXECUTE_CHUNK_SECONDS.start_timer();

        let num_txns = txn_list_with_proof.transactions.len();
        let first_version_in_request = txn_list_with_proof.first_transaction_version;
        let (_persisted_view, latest_view) = self.commit_queue.lock().persisted_and_latest_view();

        // Verify input transaction list.
        txn_list_with_proof.verify(verified_target_li.ledger_info(), first_version_in_request)?;

        // Skip transactions already in ledger.
        let txns_to_skip = txn_list_with_proof.proof.verify_extends_ledger(
            latest_view.txn_accumulator().num_leaves(),
            latest_view.txn_accumulator().root_hash(),
            first_version_in_request,
        )?;
        let mut transactions = txn_list_with_proof.transactions;
        transactions.drain(..txns_to_skip as usize);
        if txns_to_skip == num_txns {
            info!(
                "Skipping all transactions in the given chunk! Num transactions: {:?}",
                num_txns
            );
        }

        // Execute transactions.
        let state_view = self.state_view(&latest_view)?;
        let chunk_output = {
            let _timer = APTOS_EXECUTOR_VM_EXECUTE_CHUNK_SECONDS.start_timer();
            ChunkOutput::by_transaction_execution::<V>(transactions, state_view)?
        };
        let executed_chunk = Self::apply_chunk_output_for_state_sync(
            verified_target_li,
            epoch_change_li,
            &latest_view,
            chunk_output,
            &txn_list_with_proof.proof.transaction_infos[txns_to_skip..],
        )?;

        // Add result to commit queue.
        self.commit_queue.lock().enqueue(executed_chunk);

        info!(
            LogSchema::new(LogEntry::ChunkExecutor)
                .local_synced_version(latest_view.version().unwrap_or(0))
                .first_version_in_request(first_version_in_request)
                .num_txns_in_request(num_txns),
            "Executed transaction chunk!",
        );

        Ok(())
    }

    fn apply_chunk(
        &self,
        txn_output_list_with_proof: TransactionOutputListWithProof,
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()> {
        let _timer = APTOS_EXECUTOR_APPLY_CHUNK_SECONDS.start_timer();

        let num_txns = txn_output_list_with_proof.transactions_and_outputs.len();
        let first_version_in_request = txn_output_list_with_proof.first_transaction_output_version;
        let (_persisted_view, latest_view) = self.commit_queue.lock().persisted_and_latest_view();

        // Verify input transaction list.
        txn_output_list_with_proof
            .verify(verified_target_li.ledger_info(), first_version_in_request)?;

        // Skip transactions already in ledger.
        let txns_to_skip = txn_output_list_with_proof.proof.verify_extends_ledger(
            latest_view.txn_accumulator().num_leaves(),
            latest_view.txn_accumulator().root_hash(),
            first_version_in_request,
        )?;
        let mut txns_and_outputs = txn_output_list_with_proof.transactions_and_outputs;
        txns_and_outputs.drain(..txns_to_skip as usize);

        // Apply transaction outputs.
        let state_view = self.state_view(&latest_view)?;
        let chunk_output = ChunkOutput::by_transaction_output(txns_and_outputs, state_view)?;
        let executed_chunk = Self::apply_chunk_output_for_state_sync(
            verified_target_li,
            epoch_change_li,
            &latest_view,
            chunk_output,
            &txn_output_list_with_proof.proof.transaction_infos[txns_to_skip..],
        )?;

        // Add result to commit queue.
        self.commit_queue.lock().enqueue(executed_chunk);

        info!(
            LogSchema::new(LogEntry::ChunkExecutor)
                .local_synced_version(latest_view.version().unwrap_or(0))
                .first_version_in_request(first_version_in_request)
                .num_txns_in_request(num_txns),
            "Applied transaction output chunk!",
        );

        Ok(())
    }

    fn commit_chunk(&self) -> Result<ChunkCommitNotification> {
        let _timer = APTOS_EXECUTOR_COMMIT_CHUNK_SECONDS.start_timer();
        let executed_chunk = self.commit_chunk_impl()?;
        Ok(ChunkCommitNotification {
            committed_events: executed_chunk.events_to_commit(),
            committed_transactions: executed_chunk.transactions(),
            reconfiguration_occurred: executed_chunk.has_reconfiguration(),
        })
    }
}

impl<V: VMExecutor> TransactionReplayer for ChunkExecutor<V> {
    fn replay(
        &self,
        transactions: Vec<Transaction>,
        transaction_infos: Vec<TransactionInfo>,
    ) -> Result<()> {
        self.maybe_initialize()?;
        self.inner
            .read()
            .as_ref()
            .expect("not reset")
            .replay(transactions, transaction_infos)
    }

    fn commit(&self) -> Result<Arc<ExecutedChunk>> {
        self.inner.read().as_ref().expect("not reset").commit()
    }
}

impl<V: VMExecutor> TransactionReplayer for ChunkExecutorInner<V> {
    fn replay(
        &self,
        transactions: Vec<Transaction>,
        mut transaction_infos: Vec<TransactionInfo>,
    ) -> Result<()> {
        let (_persisted_view, mut latest_view) =
            self.commit_queue.lock().persisted_and_latest_view();

        let mut executed_chunk = ExecutedChunk::default();
        let mut to_run = Some(transactions);
        while !to_run.as_ref().unwrap().is_empty() {
            // Execute transactions.
            let state_view = self.state_view(&latest_view)?;
            let txns = to_run.take().unwrap();
            let (executed, to_discard, to_retry) =
                ChunkOutput::by_transaction_execution::<V>(txns, state_view)?
                    .apply_to_ledger(&latest_view)?;

            // Accumulate result and deal with retry
            ensure_no_discard(to_discard)?;
            let n = executed.to_commit.len();
            executed.ensure_transaction_infos_match(&transaction_infos[..n])?;
            transaction_infos.drain(..n);

            to_run = Some(to_retry);
            executed_chunk = executed_chunk.combine(executed)?;
            latest_view = executed_chunk.result_view.clone();
        }

        // Add result to commit queue.
        self.commit_queue.lock().enqueue(executed_chunk);

        Ok(())
    }

    fn commit(&self) -> Result<Arc<ExecutedChunk>> {
        self.commit_chunk_impl()
    }
}
