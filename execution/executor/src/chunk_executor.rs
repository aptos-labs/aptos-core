// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
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
use aptos_executor_types::{
    ChunkCommitNotification, ChunkExecutorTrait, ExecutedChunk, ParsedTransactionOutput,
    TransactionReplayer, VerifyExecutionMode,
};
use aptos_infallible::{Mutex, RwLock};
use aptos_logger::prelude::*;
use aptos_state_view::StateViewId;
use aptos_storage_interface::{
    async_proof_fetcher::AsyncProofFetcher, cached_state_view::CachedStateView, DbReaderWriter,
    ExecutedTrees,
};
use aptos_types::{
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    transaction::{
        Transaction, TransactionInfo, TransactionListWithProof, TransactionOutput,
        TransactionOutputListWithProof, TransactionStatus, Version,
    },
    write_set::WriteSet,
};
use aptos_vm::VMExecutor;
use fail::fail_point;
use itertools::multizip;
use std::{iter::once, marker::PhantomData, sync::Arc};

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
            Arc::new(AsyncProofFetcher::new(self.db.reader.clone())),
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
            chunk_output.apply_to_ledger(latest_view, None)?;
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

        let (txn_info_list_with_proof, txns_to_skip, transactions) = verify_chunk(
            txn_list_with_proof,
            verified_target_li,
            first_version_in_request,
            &latest_view,
            num_txns,
        )?;

        // Execute transactions.
        let state_view = self.state_view(&latest_view)?;
        let chunk_output = {
            let _timer = APTOS_EXECUTOR_VM_EXECUTE_CHUNK_SECONDS.start_timer();
            // State sync executor shouldn't have block gas limit.
            ChunkOutput::by_transaction_execution::<V>(transactions.into(), state_view, None)?
        };
        let executed_chunk = Self::apply_chunk_output_for_state_sync(
            verified_target_li,
            epoch_change_li,
            &latest_view,
            chunk_output,
            &txn_info_list_with_proof.transaction_infos[txns_to_skip..],
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
        txns_and_outputs.drain(..txns_to_skip);

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

/// Verifies the transaction list proof against the ledger info and returns transactions
/// that are not already applied in the ledger.
#[cfg(not(feature = "consensus-only-perf-test"))]
fn verify_chunk(
    txn_list_with_proof: TransactionListWithProof,
    verified_target_li: &LedgerInfoWithSignatures,
    first_version_in_request: Option<u64>,
    latest_view: &ExecutedTrees,
    num_txns: usize,
) -> Result<
    (
        aptos_types::proof::TransactionInfoListWithProof,
        usize,
        Vec<Transaction>,
    ),
    anyhow::Error,
> {
    // Verify input transaction list
    txn_list_with_proof.verify(verified_target_li.ledger_info(), first_version_in_request)?;

    let txn_list = txn_list_with_proof.transactions;
    let txn_info_with_proof = txn_list_with_proof.proof;

    // Skip transactions already in ledger
    let txns_to_skip = txn_info_with_proof.verify_extends_ledger(
        latest_view.txn_accumulator().num_leaves(),
        latest_view.txn_accumulator().root_hash(),
        first_version_in_request,
    )?;

    let mut transactions = txn_list;
    transactions.drain(..txns_to_skip);
    if txns_to_skip == num_txns {
        info!(
            "Skipping all transactions in the given chunk! Num transactions: {:?}",
            num_txns
        );
    }

    Ok((txn_info_with_proof, txns_to_skip, transactions))
}

/// In consensus-only mode, the [TransactionListWithProof](transaction list) is *not*
/// verified against the proof and the [LedgerInfoWithSignatures](ledger info).
/// This is because the [FakeAptosDB] from where these transactions come from
/// returns an empty proof and not an actual proof, so proof verification will
/// fail regardless. This function does not skip any transactions that may be
/// already in the ledger, because it is not necessary as execution is disabled.
#[cfg(feature = "consensus-only-perf-test")]
fn verify_chunk(
    txn_list_with_proof: TransactionListWithProof,
    _verified_target_li: &LedgerInfoWithSignatures,
    _first_version_in_request: Option<u64>,
    _latest_view: &ExecutedTrees,
    _num_txns: usize,
) -> Result<
    (
        aptos_types::proof::TransactionInfoListWithProof,
        usize,
        Vec<Transaction>,
    ),
    anyhow::Error,
> {
    // no-op: we do not verify the proof in consensus-only mode
    Ok((
        txn_list_with_proof.proof,
        0,
        txn_list_with_proof.transactions,
    ))
}

impl<V: VMExecutor> TransactionReplayer for ChunkExecutor<V> {
    fn replay(
        &self,
        transactions: Vec<Transaction>,
        transaction_infos: Vec<TransactionInfo>,
        write_sets: Vec<WriteSet>,
        event_vecs: Vec<Vec<ContractEvent>>,
        verify_execution_mode: &VerifyExecutionMode,
    ) -> Result<()> {
        self.maybe_initialize()?;
        self.inner.read().as_ref().expect("not reset").replay(
            transactions,
            transaction_infos,
            write_sets,
            event_vecs,
            verify_execution_mode,
        )
    }

    fn commit(&self) -> Result<Arc<ExecutedChunk>> {
        self.inner.read().as_ref().expect("not reset").commit()
    }
}

impl<V: VMExecutor> TransactionReplayer for ChunkExecutorInner<V> {
    fn replay(
        &self,
        mut transactions: Vec<Transaction>,
        mut transaction_infos: Vec<TransactionInfo>,
        mut write_sets: Vec<WriteSet>,
        mut event_vecs: Vec<Vec<ContractEvent>>,
        verify_execution_mode: &VerifyExecutionMode,
    ) -> Result<()> {
        let (_parent_view, mut latest_view) = self.commit_queue.lock().persisted_and_latest_view();
        let chunk_begin = latest_view.num_transactions() as Version;
        let chunk_end = chunk_begin + transactions.len() as Version; // right-exclusive

        // Find epoch boundaries.
        let mut epochs = Vec::new();
        let mut epoch_begin = chunk_begin; // epoch begin version
        for (version, events) in multizip((chunk_begin..chunk_end, event_vecs.iter())) {
            let is_epoch_ending = ParsedTransactionOutput::parse_reconfig_events(events)
                .next()
                .is_some();
            if is_epoch_ending {
                epochs.push((epoch_begin, version + 1));
                epoch_begin = version + 1;
            }
        }
        if epoch_begin < chunk_end {
            epochs.push((epoch_begin, chunk_end));
        }

        let mut executed_chunk = ExecutedChunk::default();
        // Replay epoch by epoch.
        for (begin, end) in epochs {
            self.remove_and_replay_epoch(
                &mut executed_chunk,
                &mut latest_view,
                &mut transactions,
                &mut transaction_infos,
                &mut write_sets,
                &mut event_vecs,
                begin,
                end,
                verify_execution_mode,
            )?;
        }

        self.commit_queue.lock().enqueue(executed_chunk);
        Ok(())
    }

    fn commit(&self) -> Result<Arc<ExecutedChunk>> {
        self.commit_chunk_impl()
    }
}

impl<V: VMExecutor> ChunkExecutorInner<V> {
    /// Remove `end_version - begin_version` transactions from the mutable input arguments and replay.
    /// The input range indicated by `[begin_version, end_version]` is guaranteed not to cross epoch boundaries.
    /// Notice there can be known broken versions inside the range.
    fn remove_and_replay_epoch(
        &self,
        executed_chunk: &mut ExecutedChunk,
        latest_view: &mut ExecutedTrees,
        transactions: &mut Vec<Transaction>,
        transaction_infos: &mut Vec<TransactionInfo>,
        write_sets: &mut Vec<WriteSet>,
        event_vecs: &mut Vec<Vec<ContractEvent>>,
        begin_version: Version,
        end_version: Version,
        verify_execution_mode: &VerifyExecutionMode,
    ) -> Result<()> {
        // we try to apply the txns in sub-batches split by known txns to skip and the end of the batch
        let txns_to_skip = verify_execution_mode.txns_to_skip();
        let mut batch_ends = txns_to_skip
            .range(begin_version..end_version)
            .chain(once(&end_version));

        let mut batch_begin = begin_version;
        let mut batch_end = *batch_ends.next().unwrap();
        while batch_begin < end_version {
            if batch_begin == batch_end {
                // batch_end is a known broken version that won't pass execution verification
                self.remove_and_apply(
                    executed_chunk,
                    latest_view,
                    transactions,
                    transaction_infos,
                    write_sets,
                    event_vecs,
                    batch_begin,
                    batch_begin + 1,
                )?;
                info!(
                    version_skipped = batch_begin,
                    "Skipped known broken transaction, applied transaction output directly."
                );
                batch_begin += 1;
                batch_end = *batch_ends.next().unwrap();
                continue;
            }

            // Try to run the transactions with the VM
            let next_begin = if verify_execution_mode.should_verify() {
                self.verify_execution(
                    latest_view,
                    transactions,
                    transaction_infos,
                    write_sets,
                    event_vecs,
                    batch_begin,
                    batch_end,
                    verify_execution_mode,
                )?
            } else {
                batch_end
            };
            self.remove_and_apply(
                executed_chunk,
                latest_view,
                transactions,
                transaction_infos,
                write_sets,
                event_vecs,
                batch_begin,
                next_begin,
            )?;
            batch_begin = next_begin;
        }

        Ok(())
    }

    fn verify_execution(
        &self,
        latest_view: &mut ExecutedTrees,
        transactions: &[Transaction],
        transaction_infos: &[TransactionInfo],
        write_sets: &[WriteSet],
        event_vecs: &[Vec<ContractEvent>],
        begin_version: Version,
        end_version: Version,
        verify_execution_mode: &VerifyExecutionMode,
    ) -> Result<Version> {
        // Execute transactions.
        let state_view = self.state_view(latest_view)?;
        let txns = transactions
            .iter()
            .take((end_version - begin_version) as usize)
            .cloned()
            .collect::<Vec<Transaction>>();

        // State sync executor shouldn't have block gas limit.
        let chunk_output =
            ChunkOutput::by_transaction_execution::<V>(txns.into(), state_view, None)?;
        // not `zip_eq`, deliberately
        for (version, txn_out, txn_info, write_set, events) in multizip((
            begin_version..end_version,
            chunk_output.transaction_outputs.iter(),
            transaction_infos.iter(),
            write_sets.iter(),
            event_vecs.iter(),
        )) {
            if let Err(err) = txn_out.ensure_match_transaction_info(
                version,
                txn_info,
                Some(write_set),
                Some(events),
            ) {
                if verify_execution_mode.is_lazy_quit() {
                    error!("(Not quitting right away.) {}", err);
                    verify_execution_mode.mark_seen_error();
                    return Ok(version + 1);
                } else {
                    return Err(err);
                }
            }
        }
        Ok(end_version)
    }

    /// Consume `end_version - begin_version` txns from the mutable input arguments
    /// It's guaranteed that there's no known broken versions or epoch endings in the range.
    fn remove_and_apply(
        &self,
        executed_chunk: &mut ExecutedChunk,
        latest_view: &mut ExecutedTrees,
        transactions: &mut Vec<Transaction>,
        transaction_infos: &mut Vec<TransactionInfo>,
        write_sets: &mut Vec<WriteSet>,
        event_vecs: &mut Vec<Vec<ContractEvent>>,
        begin_version: Version,
        end_version: Version,
    ) -> Result<()> {
        let num_txns = (end_version - begin_version) as usize;
        let txn_infos: Vec<_> = transaction_infos.drain(..num_txns).collect();
        let txns_and_outputs = multizip((
            transactions.drain(..num_txns),
            txn_infos.iter(),
            write_sets.drain(..num_txns),
            event_vecs.drain(..num_txns),
        ))
        .map(|(txn, txn_info, write_set, events)| {
            (
                txn,
                TransactionOutput::new(
                    write_set,
                    events,
                    txn_info.gas_used(),
                    TransactionStatus::Keep(txn_info.status().clone()),
                ),
            )
        })
        .collect();

        let state_view = self.state_view(latest_view)?;
        let chunk_output = ChunkOutput::by_transaction_output(txns_and_outputs, state_view)?;
        let (executed_batch, to_discard, to_retry) =
            chunk_output.apply_to_ledger(latest_view, None)?;
        ensure_no_discard(to_discard)?;
        ensure_no_retry(to_retry)?;
        executed_batch.ensure_transaction_infos_match(&txn_infos)?;

        executed_chunk.combine(executed_batch);
        *latest_view = executed_chunk.result_view.clone();
        Ok(())
    }
}
