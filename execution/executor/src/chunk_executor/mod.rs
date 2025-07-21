// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    logging::{LogEntry, LogSchema},
    metrics::{APPLY_CHUNK, CHUNK_OTHER_TIMERS, COMMIT_CHUNK, CONCURRENCY_GAUGE, EXECUTE_CHUNK},
    types::{
        executed_chunk::ExecutedChunk, partial_state_compute_result::PartialStateComputeResult,
    },
    workflow::{
        do_get_execution_output::DoGetExecutionOutput, do_ledger_update::DoLedgerUpdate,
        do_state_checkpoint::DoStateCheckpoint,
    },
};
use anyhow::{anyhow, ensure, Result};
use aptos_executor_types::{
    ChunkCommitNotification, ChunkExecutorTrait, TransactionReplayer, VerifyExecutionMode,
};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_infallible::{Mutex, RwLock};
use aptos_logger::prelude::*;
use aptos_metrics_core::{IntGaugeHelper, TimerHelper};
use aptos_storage_interface::{
    state_store::{
        state::State, state_summary::ProvableStateSummary,
        state_view::cached_state_view::CachedStateView,
    },
    DbReaderWriter,
};
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain,
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    state_store::StateViewId,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo,
        PersistedAuxiliaryInfo, Transaction, TransactionAuxiliaryData, TransactionInfo,
        TransactionListWithProof, TransactionListWithProofV2, TransactionOutput,
        TransactionOutputListWithProof, TransactionOutputListWithProofV2, TransactionStatus,
        Version,
    },
    write_set::WriteSet,
};
use aptos_vm::VMBlockExecutor;
use chunk_commit_queue::{ChunkCommitQueue, ChunkToUpdateLedger};
use chunk_result_verifier::{ChunkResultVerifier, ReplayChunkVerifier, StateSyncChunkVerifier};
use fail::fail_point;
use itertools::{multizip, Itertools};
use std::{
    iter::once,
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};
use transaction_chunk::{ChunkToApply, ChunkToExecute, TransactionChunk};

pub mod chunk_commit_queue;
pub mod chunk_result_verifier;
pub mod transaction_chunk;

pub struct ChunkExecutor<V> {
    db: DbReaderWriter,
    inner: RwLock<Option<ChunkExecutorInner<V>>>,
}

impl<V: VMBlockExecutor> ChunkExecutor<V> {
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

    fn with_inner<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&ChunkExecutorInner<V>) -> Result<T>,
    {
        let locked = self.inner.read();
        let inner = locked.as_ref().expect("not reset");

        let has_pending_pre_commit = inner.has_pending_pre_commit.load(Ordering::Acquire);
        f(inner).map_err(|error| {
            if has_pending_pre_commit {
                panic!(
                    "Hit error with pending pre-committed ledger, panicking. {:?}",
                    error,
                );
            }
            error
        })
    }

    pub fn is_empty(&self) -> bool {
        self.with_inner(|inner| Ok(inner.is_empty())).unwrap()
    }
}

impl<V: VMBlockExecutor> ChunkExecutorTrait for ChunkExecutor<V> {
    fn enqueue_chunk_by_execution(
        &self,
        txn_list_with_proof: TransactionListWithProofV2,
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["chunk", "enqueue_by_execution"]);
        let _timer = EXECUTE_CHUNK.start_timer();

        self.maybe_initialize()?;

        // Verify input data.
        // In consensus-only mode, txn_list_with_proof is fake.
        if !cfg!(feature = "consensus-only-perf-test") {
            txn_list_with_proof.verify(
                verified_target_li.ledger_info(),
                txn_list_with_proof
                    .get_transaction_list_with_proof()
                    .first_transaction_version,
            )?;
        }

        let (txn_list_with_proof, persisted_aux_info) = txn_list_with_proof.into_parts();
        // Compose enqueue_chunk parameters.
        let TransactionListWithProof {
            transactions,
            events: _,
            first_transaction_version: v,
            proof: txn_infos_with_proof,
        } = txn_list_with_proof;

        let chunk = ChunkToExecute {
            transactions,
            persisted_aux_info,
            first_version: v.ok_or_else(|| anyhow!("first version is None"))?,
        };
        let chunk_verifier = Arc::new(StateSyncChunkVerifier {
            txn_infos_with_proof,
            verified_target_li: verified_target_li.clone(),
            epoch_change_li: epoch_change_li.cloned(),
        });

        // Call the shared implementation.
        self.with_inner(|inner| inner.enqueue_chunk(chunk, chunk_verifier, "execute"))
    }

    fn enqueue_chunk_by_transaction_outputs(
        &self,
        txn_output_list_with_proof: TransactionOutputListWithProofV2,
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["chunk", "enqueue_by_outputs"]);
        let _timer = APPLY_CHUNK.start_timer();

        // Verify input data.
        THREAD_MANAGER.get_exe_cpu_pool().install(|| {
            let _timer = CHUNK_OTHER_TIMERS.timer_with(&["apply_chunk__verify"]);
            txn_output_list_with_proof.verify(
                verified_target_li.ledger_info(),
                txn_output_list_with_proof
                    .get_output_list_with_proof()
                    .first_transaction_output_version,
            )
        })?;

        let (txn_output_list_with_proof, persisted_aux_info) =
            txn_output_list_with_proof.into_parts();
        // Compose enqueue_chunk parameters.
        let TransactionOutputListWithProof {
            transactions_and_outputs,
            first_transaction_output_version: v,
            proof: txn_infos_with_proof,
        } = txn_output_list_with_proof;
        let (transactions, transaction_outputs): (Vec<_>, Vec<_>) =
            transactions_and_outputs.into_iter().unzip();

        let chunk = ChunkToApply {
            transactions,
            transaction_outputs,
            persisted_aux_info,
            first_version: v.ok_or_else(|| anyhow!("first version is None"))?,
        };
        let chunk_verifier = Arc::new(StateSyncChunkVerifier {
            txn_infos_with_proof,
            verified_target_li: verified_target_li.clone(),
            epoch_change_li: epoch_change_li.cloned(),
        });

        // Call the shared implementation.
        self.with_inner(|inner| inner.enqueue_chunk(chunk, chunk_verifier, "apply"))
    }

    fn update_ledger(&self) -> Result<()> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["chunk", "update_ledger"]);

        self.with_inner(|inner| inner.update_ledger())
    }

    fn commit_chunk(&self) -> Result<ChunkCommitNotification> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["chunk", "commit_chunk"]);

        self.with_inner(|inner| inner.commit_chunk())
    }

    fn reset(&self) -> Result<()> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["chunk", "reset"]);

        *self.inner.write() = Some(ChunkExecutorInner::new(self.db.clone())?);
        Ok(())
    }

    fn finish(&self) {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["chunk", "finish"]);

        *self.inner.write() = None;
    }
}

struct ChunkExecutorInner<V> {
    db: DbReaderWriter,
    commit_queue: Mutex<ChunkCommitQueue>,
    has_pending_pre_commit: AtomicBool,
    _phantom: PhantomData<V>,
}

impl<V: VMBlockExecutor> ChunkExecutorInner<V> {
    pub fn new(db: DbReaderWriter) -> Result<Self> {
        let commit_queue = ChunkCommitQueue::new_from_db(&db.reader)?;

        let next_pre_committed_version = commit_queue.expecting_version();
        let next_synced_version = db.reader.get_synced_version()?.map_or(0, |v| v + 1);
        assert!(next_synced_version <= next_pre_committed_version);
        let has_pending_pre_commit = next_synced_version < next_pre_committed_version;

        Ok(Self {
            db,
            commit_queue: Mutex::new(commit_queue),
            has_pending_pre_commit: AtomicBool::new(has_pending_pre_commit),
            _phantom: PhantomData,
        })
    }

    fn state_view(&self, state: &State) -> Result<CachedStateView> {
        let first_version = state.next_version();
        Ok(CachedStateView::new(
            StateViewId::ChunkExecution { first_version },
            self.db.reader.clone(),
            state.clone(),
        )?)
    }

    fn commit_chunk_impl(&self) -> Result<ExecutedChunk> {
        let _timer = CHUNK_OTHER_TIMERS.timer_with(&["commit_chunk_impl__total"]);
        let chunk = {
            let _timer =
                CHUNK_OTHER_TIMERS.timer_with(&["commit_chunk_impl__next_chunk_to_commit"]);
            self.commit_queue.lock().next_chunk_to_commit()?
        };

        let output = chunk.output.expect_complete_result();
        let num_txns = output.num_transactions_to_commit();
        if chunk.ledger_info_opt.is_some() || num_txns != 0 {
            let _timer = CHUNK_OTHER_TIMERS.timer_with(&["commit_chunk_impl__save_txns"]);
            // TODO(aldenhu): remove since there's no practical strategy to recover from this error.
            fail_point!("executor::commit_chunk", |_| {
                Err(anyhow::anyhow!("Injected error in commit_chunk"))
            });
            self.db.writer.save_transactions(
                output.as_chunk_to_commit(),
                chunk.ledger_info_opt.as_ref(),
                false, // sync_commit
            )?;
        }

        let _timer = CHUNK_OTHER_TIMERS.timer_with(&["commit_chunk_impl__dequeue_and_return"]);
        self.commit_queue.lock().dequeue_committed()?;

        Ok(chunk)
    }

    fn is_empty(&self) -> bool {
        self.commit_queue.lock().is_empty()
    }

    // ************************* Chunk Executor Implementation *************************
    fn enqueue_chunk<Chunk: TransactionChunk + Sync>(
        &self,
        chunk: Chunk,
        chunk_verifier: Arc<dyn ChunkResultVerifier + Send + Sync>,
        mode_for_log: &'static str,
    ) -> Result<()> {
        let parent_state = self.commit_queue.lock().latest_state().clone();

        let first_version = parent_state.next_version();
        ensure!(
            chunk.first_version() == parent_state.next_version(),
            "Chunk carries unexpected first version. Expected: {}, got: {}",
            parent_state.next_version(),
            chunk.first_version(),
        );

        let num_txns = chunk.len();

        let state_view = self.state_view(parent_state.latest())?;
        let execution_output = chunk.into_output::<V>(&parent_state, state_view)?;
        let output = PartialStateComputeResult::new(execution_output);

        // Enqueue for next stage.
        self.commit_queue
            .lock()
            .enqueue_for_ledger_update(ChunkToUpdateLedger {
                output,
                chunk_verifier,
            })?;

        info!(
            LogSchema::new(LogEntry::ChunkExecutor)
                .first_version_in_request(Some(first_version))
                .num_txns_in_request(num_txns),
            mode = mode_for_log,
            "Enqueued transaction chunk!",
        );

        Ok(())
    }

    pub fn update_ledger(&self) -> Result<()> {
        let _timer = CHUNK_OTHER_TIMERS.timer_with(&["chunk_update_ledger_total"]);

        let (parent_state_summary, parent_accumulator, chunk) =
            self.commit_queue.lock().next_chunk_to_update_ledger()?;
        let ChunkToUpdateLedger {
            output,
            chunk_verifier,
        } = chunk;

        let state_checkpoint_output = DoStateCheckpoint::run(
            &output.execution_output,
            &parent_state_summary,
            &ProvableStateSummary::new_persisted(self.db.reader.as_ref())?,
            Some(
                chunk_verifier
                    .transaction_infos()
                    .iter()
                    .map(|t| t.state_checkpoint_hash())
                    .collect_vec(),
            ),
        )?;

        let ledger_update_output = DoLedgerUpdate::run(
            &output.execution_output,
            &state_checkpoint_output,
            parent_accumulator.clone(),
        )?;

        chunk_verifier.verify_chunk_result(&parent_accumulator, &ledger_update_output)?;

        let ledger_info_opt = chunk_verifier.maybe_select_chunk_ending_ledger_info(
            &ledger_update_output,
            output.execution_output.next_epoch_state.as_ref(),
        )?;
        output.set_state_checkpoint_output(state_checkpoint_output);
        output.set_ledger_update_output(ledger_update_output);

        let first_version = output.execution_output.first_version;
        let num_txns = output.execution_output.num_transactions_to_commit();
        let executed_chunk = ExecutedChunk {
            output,
            ledger_info_opt,
        };

        self.commit_queue
            .lock()
            .save_ledger_update_output(executed_chunk)?;

        info!(
            LogSchema::new(LogEntry::ChunkExecutor)
                .first_version_in_request(Some(first_version))
                .num_txns_in_request(num_txns),
            "Calculated ledger update!",
        );
        Ok(())
    }

    fn commit_chunk(&self) -> Result<ChunkCommitNotification> {
        let _timer = COMMIT_CHUNK.start_timer();
        let executed_chunk = self.commit_chunk_impl()?;
        self.has_pending_pre_commit.store(false, Ordering::Release);

        let commit_notification = {
            let _timer =
                CHUNK_OTHER_TIMERS.timer_with(&["commit_chunk__into_chunk_commit_notification"]);
            executed_chunk
                .output
                .expect_complete_result()
                .make_chunk_commit_notification()
        };

        Ok(commit_notification)
    }
}

impl<V: VMBlockExecutor> TransactionReplayer for ChunkExecutor<V> {
    fn enqueue_chunks(
        &self,
        transactions: Vec<Transaction>,
        persisted_aux_info: Vec<PersistedAuxiliaryInfo>,
        transaction_infos: Vec<TransactionInfo>,
        write_sets: Vec<WriteSet>,
        event_vecs: Vec<Vec<ContractEvent>>,
        verify_execution_mode: &VerifyExecutionMode,
    ) -> Result<usize> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["replayer", "replay"]);

        self.maybe_initialize()?;
        self.inner
            .read()
            .as_ref()
            .expect("not reset")
            .enqueue_chunks(
                transactions,
                persisted_aux_info,
                transaction_infos,
                write_sets,
                event_vecs,
                verify_execution_mode,
            )
    }

    fn commit(&self) -> Result<Version> {
        let _guard = CONCURRENCY_GAUGE.concurrency_with(&["replayer", "commit"]);

        self.inner.read().as_ref().expect("not reset").commit()
    }
}

impl<V: VMBlockExecutor> ChunkExecutorInner<V> {
    fn enqueue_chunks(
        &self,
        mut transactions: Vec<Transaction>,
        mut persisted_aux_info: Vec<PersistedAuxiliaryInfo>,
        mut transaction_infos: Vec<TransactionInfo>,
        mut write_sets: Vec<WriteSet>,
        mut event_vecs: Vec<Vec<ContractEvent>>,
        verify_execution_mode: &VerifyExecutionMode,
    ) -> Result<usize> {
        let started = Instant::now();
        let num_txns = transactions.len();
        let chunk_begin = self.commit_queue.lock().expecting_version();
        let chunk_end = chunk_begin + num_txns as Version; // right-exclusive

        // Find epoch boundaries.
        let mut epochs = Vec::new();
        let mut epoch_begin = chunk_begin; // epoch begin version
        for (version, events) in multizip((chunk_begin..chunk_end, event_vecs.iter())) {
            let is_epoch_ending = events.iter().any(ContractEvent::is_new_epoch_event);
            if is_epoch_ending {
                epochs.push((epoch_begin, version + 1));
                epoch_begin = version + 1;
            }
        }
        if epoch_begin < chunk_end {
            epochs.push((epoch_begin, chunk_end));
        }

        let mut chunks_enqueued = 0;
        // Replay epoch by epoch.
        for (begin, end) in epochs {
            chunks_enqueued += self.remove_and_replay_epoch(
                &mut transactions,
                &mut persisted_aux_info,
                &mut transaction_infos,
                &mut write_sets,
                &mut event_vecs,
                begin,
                end,
                verify_execution_mode,
            )?;
        }

        info!(
            num_txns = num_txns,
            tps = (num_txns as f64 / started.elapsed().as_secs_f64()),
            "TransactionReplayer::replay() OK"
        );

        Ok(chunks_enqueued)
    }

    fn commit(&self) -> Result<Version> {
        let started = Instant::now();

        let chunk = self.commit_chunk_impl()?;
        let output = chunk.output.expect_complete_result();

        let num_committed = output.num_transactions_to_commit();
        info!(
            num_committed = num_committed,
            tps = num_committed as f64 / started.elapsed().as_secs_f64(),
            "TransactionReplayer::commit() OK"
        );

        Ok(output.expect_last_version())
    }

    /// Remove `end_version - begin_version` transactions from the mutable input arguments and replay.
    /// The input range indicated by `[begin_version, end_version]` is guaranteed not to cross epoch boundaries.
    /// Notice there can be known broken versions inside the range.
    fn remove_and_replay_epoch(
        &self,
        transactions: &mut Vec<Transaction>,
        persisted_aux_info: &mut Vec<PersistedAuxiliaryInfo>,
        transaction_infos: &mut Vec<TransactionInfo>,
        write_sets: &mut Vec<WriteSet>,
        event_vecs: &mut Vec<Vec<ContractEvent>>,
        begin_version: Version,
        end_version: Version,
        verify_execution_mode: &VerifyExecutionMode,
    ) -> Result<usize> {
        // we try to apply the txns in sub-batches split by known txns to skip and the end of the batch
        let txns_to_skip = verify_execution_mode.txns_to_skip();
        let mut batch_ends = txns_to_skip
            .range(begin_version..end_version)
            .chain(once(&end_version));

        let mut chunks_enqueued = 0;

        let mut batch_begin = begin_version;
        let mut batch_end = *batch_ends.next().unwrap();
        while batch_begin < end_version {
            if batch_begin == batch_end {
                // batch_end is a known broken version that won't pass execution verification
                self.remove_and_apply(
                    transactions,
                    persisted_aux_info,
                    transaction_infos,
                    write_sets,
                    event_vecs,
                    batch_begin,
                    batch_begin + 1,
                )?;
                chunks_enqueued += 1;
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
                transactions,
                persisted_aux_info,
                transaction_infos,
                write_sets,
                event_vecs,
                batch_begin,
                next_begin,
            )?;
            chunks_enqueued += 1;
            batch_begin = next_begin;
        }

        Ok(chunks_enqueued)
    }

    fn verify_execution(
        &self,
        transactions: &[Transaction],
        transaction_infos: &[TransactionInfo],
        write_sets: &[WriteSet],
        event_vecs: &[Vec<ContractEvent>],
        begin_version: Version,
        end_version: Version,
        verify_execution_mode: &VerifyExecutionMode,
    ) -> Result<Version> {
        // Execute transactions.
        let parent_state = self.commit_queue.lock().latest_state().clone();
        let state_view = self.state_view(parent_state.latest())?;
        let txns = transactions
            .iter()
            .take((end_version - begin_version) as usize)
            .cloned()
            .map(|t| t.into())
            .collect::<Vec<SignatureVerifiedTransaction>>();

        let mut auxiliary_info = Vec::new();
        // TODO(grao): Pass in persisted auxiliary info.
        auxiliary_info.resize(txns.len(), AuxiliaryInfo::new_empty());
        // State sync executor shouldn't have block gas limit.
        let execution_output = DoGetExecutionOutput::by_transaction_execution::<V>(
            &V::new(),
            txns.into(),
            auxiliary_info,
            &parent_state,
            state_view,
            BlockExecutorConfigFromOnchain::new_no_block_limit(),
            TransactionSliceMetadata::chunk(begin_version, end_version),
        )?;
        // not `zip_eq`, deliberately
        for (version, txn_out, txn_info, write_set, events) in multizip((
            begin_version..end_version,
            &execution_output.to_commit.transaction_outputs,
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
                return if verify_execution_mode.is_lazy_quit() {
                    error!("(Not quitting right away.) {}", err);
                    verify_execution_mode.mark_seen_error();
                    Ok(version + 1)
                } else {
                    Err(err)
                };
            }
        }
        Ok(end_version)
    }

    /// Consume `end_version - begin_version` txns from the mutable input arguments
    /// It's guaranteed that there's no known broken versions or epoch endings in the range.
    fn remove_and_apply(
        &self,
        transactions: &mut Vec<Transaction>,
        persisted_aux_info: &mut Vec<PersistedAuxiliaryInfo>,
        transaction_infos: &mut Vec<TransactionInfo>,
        write_sets: &mut Vec<WriteSet>,
        event_vecs: &mut Vec<Vec<ContractEvent>>,
        begin_version: Version,
        end_version: Version,
    ) -> Result<()> {
        let num_txns = (end_version - begin_version) as usize;
        let txn_infos: Vec<_> = transaction_infos.drain(..num_txns).collect();
        let (transactions, persisted_aux_info, transaction_outputs) = multizip((
            transactions.drain(..num_txns),
            persisted_aux_info.drain(..num_txns),
            txn_infos.iter(),
            write_sets.drain(..num_txns),
            event_vecs.drain(..num_txns),
        ))
        .map(|(txn, persisted_aux_info, txn_info, write_set, events)| {
            (
                txn,
                persisted_aux_info,
                TransactionOutput::new(
                    write_set,
                    events,
                    txn_info.gas_used(),
                    TransactionStatus::Keep(txn_info.status().clone()),
                    TransactionAuxiliaryData::default(), // No auxiliary data if transaction is not executed through VM
                ),
            )
        })
        .multiunzip();

        let chunk = ChunkToApply {
            transactions,
            transaction_outputs,
            persisted_aux_info,
            first_version: begin_version,
        };
        let chunk_verifier = Arc::new(ReplayChunkVerifier {
            transaction_infos: txn_infos,
        });
        self.enqueue_chunk(chunk, chunk_verifier, "replay")?;

        Ok(())
    }
}
