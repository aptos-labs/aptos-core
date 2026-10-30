// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    backup_types::{
        epoch_ending::restore::EpochHistory,
        transaction::{
            analysis::TransactionAnalysis,
            manifest::{TransactionBackup, TransactionChunk, TransactionChunkFormat},
        },
    },
    metrics::{
        restore::{TRANSACTION_REPLAY_VERSION, TRANSACTION_SAVE_VERSION},
        verify::VERIFY_TRANSACTION_VERSION,
        OTHER_TIMERS_SECONDS,
    },
    storage::{BackupStorage, FileHandle},
    utils::{
        error_notes::ErrorNotes,
        read_record_bytes::ReadRecordBytes,
        storage_ext::BackupStorageExt,
        stream::{StreamX, TryStreamX},
        GlobalRestoreOptions, RestoreRunMode,
    },
};
use anyhow::{anyhow, ensure, Result};
use aptos_db::backup::restore_handler::RestoreHandler;
use aptos_executor::chunk_executor::ChunkExecutor;
use aptos_executor_types::{ChunkExecutorTrait, TransactionReplayer, VerifyExecutionMode};
use aptos_logger::prelude::*;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::{
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    proof::{TransactionAccumulatorRangeProof, TransactionInfoListWithProof},
    transaction::{
        PersistedAuxiliaryInfo, Transaction, TransactionInfo, TransactionListWithAuxiliaryInfos,
        TransactionListWithProof, TransactionListWithProofV2, Version,
    },
    write_set::WriteSet,
};
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, AptosVM};
use clap::Parser;
use futures::{
    future,
    future::TryFutureExt,
    stream,
    stream::{Peekable, Stream, TryStreamExt},
    StreamExt,
};
use itertools::{izip, Itertools};
use std::{
    cmp::{max, min},
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    time::Instant,
};
use tokio::io::BufReader;

const BATCH_SIZE: usize = if cfg!(test) { 2 } else { 10000 };

#[derive(Parser)]
pub struct TransactionRestoreOpt {
    #[clap(long = "transaction-manifest")]
    pub manifest_handle: FileHandle,
    #[clap(
        long = "replay-transactions-from-version",
        help = "Transactions with this version and above will be replayed so state and events are \
        gonna pop up. Requires state at the version right before this to exist, either by \
        recovering a state snapshot, or previous transaction replay."
    )]
    pub replay_from_version: Option<Version>,
    #[clap(long)]
    pub kv_only_replay: Option<bool>,
}

impl TransactionRestoreOpt {
    pub fn replay_from_version(&self) -> Version {
        self.replay_from_version.unwrap_or(Version::MAX)
    }
}

pub struct TransactionRestoreController {
    inner: TransactionRestoreBatchController,
}

struct LoadedChunk {
    pub manifest: TransactionChunk,
    pub txns: Vec<Transaction>,
    pub persisted_aux_info: Vec<PersistedAuxiliaryInfo>,
    pub txn_infos: Vec<TransactionInfo>,
    pub event_vecs: Vec<Vec<ContractEvent>>,
    pub write_sets: Vec<WriteSet>,
    pub range_proof: TransactionAccumulatorRangeProof,
}

impl LoadedChunk {
    async fn load(
        manifest: TransactionChunk,
        storage: &Arc<dyn BackupStorage>,
        epoch_history: Option<&Arc<EpochHistory>>,
    ) -> Result<Self> {
        let mut file = BufReader::new(storage.open_for_read(&manifest.transactions).await?);
        let mut txns = Vec::new();
        let mut persisted_aux_info = Vec::new();
        let mut txn_infos = Vec::new();
        let mut event_vecs = Vec::new();
        let mut write_sets = Vec::new();

        while let Some(record_bytes) = file.read_record_bytes().await? {
            let (txn, aux_info, txn_info, events, write_set): (
                _,
                PersistedAuxiliaryInfo,
                _,
                _,
                WriteSet,
            ) = match manifest.format {
                TransactionChunkFormat::V0 => {
                    let (txn, txn_info, events, write_set) = bcs::from_bytes(&record_bytes)?;
                    (
                        txn,
                        PersistedAuxiliaryInfo::None,
                        txn_info,
                        events,
                        write_set,
                    )
                },
                TransactionChunkFormat::V1 => bcs::from_bytes(&record_bytes)?,
            };
            txns.push(txn);
            persisted_aux_info.push(aux_info);
            txn_infos.push(txn_info);
            event_vecs.push(events);
            write_sets.push(write_set);
        }

        ensure!(
            manifest.first_version + (txns.len() as Version) == manifest.last_version + 1,
            "Number of items in chunks doesn't match that in manifest. first_version: {}, last_version: {}, items in chunk: {}",
            manifest.first_version,
            manifest.last_version,
            txns.len(),
        );

        let (range_proof, ledger_info) = storage
            .load_bcs_file::<(TransactionAccumulatorRangeProof, LedgerInfoWithSignatures)>(
                &manifest.proof,
            )
            .await?;
        if let Some(epoch_history) = epoch_history {
            epoch_history.verify_ledger_info(&ledger_info)?;
        }

        // make a `TransactionListWithProof` to reuse its verification code.
        let txn_list_with_proof =
            TransactionListWithProofV2::new(TransactionListWithAuxiliaryInfos::new(
                TransactionListWithProof::new(
                    txns,
                    Some(event_vecs),
                    Some(manifest.first_version),
                    TransactionInfoListWithProof::new(range_proof, txn_infos),
                ),
                persisted_aux_info,
            ));
        txn_list_with_proof.verify(ledger_info.ledger_info(), Some(manifest.first_version))?;
        // and disassemble it to get things back.
        let (txn_list_with_proof, persisted_aux_info) = txn_list_with_proof.into_parts();
        let txns = txn_list_with_proof.transactions;
        let range_proof = txn_list_with_proof
            .proof
            .ledger_info_to_transaction_infos_proof;
        let txn_infos = txn_list_with_proof.proof.transaction_infos;
        let event_vecs = txn_list_with_proof.events.expect("unknown to be Some.");

        Ok(Self {
            manifest,
            txns,
            persisted_aux_info,
            txn_infos,
            event_vecs,
            range_proof,
            write_sets,
        })
    }

    fn unpack(
        self,
    ) -> (
        Vec<Transaction>,
        Vec<PersistedAuxiliaryInfo>,
        Vec<TransactionInfo>,
        Vec<Vec<ContractEvent>>,
        Vec<WriteSet>,
    ) {
        let Self {
            manifest: _,
            txns,
            persisted_aux_info,
            txn_infos,
            event_vecs,
            write_sets,
            range_proof: _,
        } = self;

        (txns, persisted_aux_info, txn_infos, event_vecs, write_sets)
    }
}

impl TransactionRestoreController {
    pub fn new(
        opt: TransactionRestoreOpt,
        global_opt: GlobalRestoreOptions,
        storage: Arc<dyn BackupStorage>,
        epoch_history: Option<Arc<EpochHistory>>,
        verify_execution_mode: VerifyExecutionMode,
    ) -> Self {
        let replay_from_version: Option<(Version, bool)> =
            if let Some(version) = opt.replay_from_version {
                Some((
                    version,
                    opt.kv_only_replay.expect(
                        "KV only replay must be specified if replay from version is specified.",
                    ),
                ))
            } else {
                None
            };
        let inner = TransactionRestoreBatchController::new(
            global_opt,
            storage,
            vec![opt.manifest_handle],
            None,
            replay_from_version,
            epoch_history,
            verify_execution_mode,
            None,
        );

        Self { inner }
    }

    pub async fn run(self) -> Result<()> {
        self.inner.run().await
    }
}

impl TransactionRestoreController {}

/// Takes a series of transaction backup manifests, preheat in parallel, then execute in order.
pub struct TransactionRestoreBatchController {
    global_opt: GlobalRestoreOptions,
    storage: Arc<dyn BackupStorage>,
    manifest_handles: Vec<FileHandle>,
    replay_from_version: Option<(Version, bool)>,
    epoch_history: Option<Arc<EpochHistory>>,
    verify_execution_mode: VerifyExecutionMode,
    output_transaction_analysis: Option<PathBuf>,
    first_version: Option<Version>,
}

impl TransactionRestoreBatchController {
    pub fn new(
        global_opt: GlobalRestoreOptions,
        storage: Arc<dyn BackupStorage>,
        manifest_handles: Vec<FileHandle>,
        first_version: Option<Version>,
        replay_from_version: Option<(Version, bool)>, // bool indicates if this is a KV only replay
        epoch_history: Option<Arc<EpochHistory>>,
        verify_execution_mode: VerifyExecutionMode,
        output_transaction_analysis: Option<PathBuf>,
    ) -> Self {
        Self {
            global_opt,
            storage,
            manifest_handles,
            replay_from_version,
            epoch_history,
            verify_execution_mode,
            output_transaction_analysis,
            first_version,
        }
    }

    pub async fn run(self) -> Result<()> {
        let name = self.name();
        info!("{} started.", name);
        self.run_impl()
            .await
            .map_err(|e| anyhow!("{} failed: {}", name, e))?;
        info!("{} succeeded.", name);
        Ok(())
    }

    fn name(&self) -> String {
        format!("transaction {}", self.global_opt.run_mode.name())
    }

    async fn run_impl(mut self) -> Result<()> {
        if self.manifest_handles.is_empty() {
            return Ok(());
        }

        // Replay can't end in the middle of a block, so restore as if the target were the last block
        // end at or before it.
        if !self.global_opt.run_mode.is_verify() {
            let Some(block_end) = self.last_block_end_at_or_before_target().await? else {
                warn!(
                    target_version = self.global_opt.target_version,
                    "No block end at or before the target version, nothing to restore."
                );
                return Ok(());
            };
            // The DB can be past it already, e.g. on a rerun after a completed restore.
            if let Some(first_version) = self.first_version.filter(|v| block_end < *v) {
                info!(
                    block_end = block_end,
                    first_version = first_version,
                    "DB is already past the last block end at or before the target version, \
                    nothing to restore."
                );
                return Ok(());
            }
            self.global_opt.target_version = block_end;
        }

        let mut loaded_chunk_stream = self.loaded_chunk_stream();
        // Always confirm (or save, if missing) the frozen subtrees of the first chunk: the
        // transaction accumulator can only be extended from a chunk that doesn't start at
        // version 0 if its left siblings are in the DB (e.g. phase 1 of RestoreCoordinator
        // passes Some(0) on a fresh DB whose backups start later). This is idempotent when
        // they are already present. The chunk's first version is the baseline only when the
        // caller didn't provide one.
        let first_chunk_version = self
            .confirm_or_save_frozen_subtrees(&mut loaded_chunk_stream)
            .await?;
        let first_version = self.first_version.unwrap_or(first_chunk_version);
        if let RestoreRunMode::Restore { restore_handler } = self.global_opt.run_mode.as_ref() {
            ensure!(
                self.output_transaction_analysis.is_none(),
                "Bug: requested to output transaction output sizing info in restore mode.",
            );
            AptosVM::set_concurrency_level_once(self.global_opt.replay_concurrency_level);

            let kv_only = self.replay_from_version.is_some_and(|(_, k)| k);
            let txns_to_execute_stream = self
                .save_before_replay_version(first_version, loaded_chunk_stream, restore_handler)
                .await?;

            if let Some(txns_to_execute_stream) = txns_to_execute_stream {
                if kv_only {
                    self.replay_kv(restore_handler, txns_to_execute_stream)
                        .await?;
                } else {
                    self.replay_transactions(restore_handler, txns_to_execute_stream)
                        .await?;
                }
            }
        } else {
            self.go_through_verified_chunks(loaded_chunk_stream, first_version)
                .await?;
        }
        Ok(())
    }

    /// Returns the last txn with a state checkpoint hash at or before the target version, searching
    /// back from the chunk with the target, or from the last chunk if the backups end before it.
    async fn last_block_end_at_or_before_target(&self) -> Result<Option<Version>> {
        let target_version = self.global_opt.target_version;
        let mut chunks = Vec::new();
        for hdl in &self.manifest_handles {
            let manifest: TransactionBackup =
                self.storage.load_json_file(hdl).await.err_notes(hdl)?;
            manifest.verify()?;
            chunks.extend(
                manifest
                    .chunks
                    .into_iter()
                    .filter(|c| c.first_version <= target_version),
            );
        }
        let Some(last_version) = chunks.last().map(|c| min(c.last_version, target_version)) else {
            return Ok(None);
        };

        for chunk in chunks.into_iter().rev() {
            let first_version = chunk.first_version;
            let chunk =
                LoadedChunk::load(chunk, &self.storage, self.epoch_history.as_ref()).await?;
            let block_end = (first_version..)
                .zip(&chunk.txn_infos)
                .take_while(|(version, _)| *version <= target_version)
                .filter(|(_, txn_info)| txn_info.has_state_checkpoint_hash())
                .last()
                .map(|(version, _)| version);
            if let Some(block_end) = block_end {
                if block_end < last_version {
                    warn!(
                        target_version = target_version,
                        block_end = block_end,
                        "Restore target is in the middle of a block, ending at the block end before it."
                    );
                }
                return Ok(Some(block_end));
            }
        }
        Ok(None)
    }

    fn loaded_chunk_stream(&self) -> Peekable<impl Stream<Item = Result<LoadedChunk>> + use<>> {
        let con = self.global_opt.concurrent_downloads;

        let manifest_handle_stream = stream::iter(self.manifest_handles.clone());

        let storage = self.storage.clone();
        let manifest_stream = manifest_handle_stream
            .map(move |hdl| {
                let storage = storage.clone();
                async move { storage.load_json_file(&hdl).await.err_notes(&hdl) }
            })
            .buffered_x(con * 3, con)
            .and_then(|m: TransactionBackup| future::ready(m.verify().map(|_| m)));

        let target_version = self.global_opt.target_version;
        let first_version = self.first_version.unwrap_or(0);
        let chunk_manifest_stream = manifest_stream
            .map_ok(|m| stream::iter(m.chunks.into_iter().map(Result::<_>::Ok)))
            .try_flatten()
            .try_filter(move |c| {
                future::ready(c.first_version <= target_version && c.last_version >= first_version)
            })
            .scan(0, |last_chunk_last_version, chunk_res| {
                let res = match &chunk_res {
                    Ok(chunk) => {
                        if *last_chunk_last_version != 0
                            && chunk.first_version != *last_chunk_last_version + 1
                        {
                            Some(Err(anyhow!(
                                "Chunk range not consecutive. expecting {}, got {}",
                                *last_chunk_last_version + 1,
                                chunk.first_version
                            )))
                        } else {
                            *last_chunk_last_version = chunk.last_version;
                            Some(chunk_res)
                        }
                    },
                    Err(_) => Some(chunk_res),
                };
                future::ready(res)
            });

        let storage = self.storage.clone();
        let epoch_history = self.epoch_history.clone();
        chunk_manifest_stream
            .and_then(move |chunk| {
                let storage = storage.clone();
                let epoch_history = epoch_history.clone();
                future::ok(async move {
                    tokio::task::spawn(async move {
                        LoadedChunk::load(chunk, &storage, epoch_history.as_ref()).await
                    })
                    .err_into::<anyhow::Error>()
                    .await
                })
            })
            .try_buffered_x(con * 2, con)
            .and_then(future::ready)
            .peekable()
    }

    async fn confirm_or_save_frozen_subtrees(
        &self,
        loaded_chunk_stream: &mut Peekable<impl Unpin + Stream<Item = Result<LoadedChunk>>>,
    ) -> Result<Version> {
        let first_chunk = Pin::new(loaded_chunk_stream)
            .peek()
            .await
            .ok_or_else(|| anyhow!("LoadedChunk stream is empty."))?
            .as_ref()
            .map_err(|e| anyhow!("Error: {}", e))?;

        if let RestoreRunMode::Restore { restore_handler } = self.global_opt.run_mode.as_ref() {
            restore_handler.confirm_or_save_frozen_subtrees(
                first_chunk.manifest.first_version,
                first_chunk.range_proof.left_siblings(),
            )?;
        }

        Ok(first_chunk.manifest.first_version)
    }

    async fn save_before_replay_version(
        &self,
        global_first_version: Version,
        loaded_chunk_stream: impl Stream<Item = Result<LoadedChunk>> + Unpin,
        restore_handler: &RestoreHandler,
    ) -> Result<
        Option<
            impl Stream<
                Item = Result<(
                    Transaction,
                    PersistedAuxiliaryInfo,
                    TransactionInfo,
                    WriteSet,
                    Vec<ContractEvent>,
                )>,
            >,
        >,
    > {
        // get the next expected transaction version of the current aptos db from txn_info CF
        let next_expected_version = self
            .global_opt
            .run_mode
            .get_next_expected_transaction_version()?;
        let start = Instant::now();

        let restore_handler_clone = restore_handler.clone();
        // DB doesn't allow replaying anything before what's in DB already.
        // self.replay_from_version is from cli argument. However, in fact, we either not replay or replay
        // after current DB's version.
        let first_to_replay = max(
            self.replay_from_version
                .map_or(Version::MAX, |(version, _)| version),
            next_expected_version,
        );
        let target_version = self.global_opt.target_version;

        let mut txns_to_execute_stream = loaded_chunk_stream
            .and_then(move |chunk| {
                let restore_handler = restore_handler_clone.clone();
                future::ok(async move {
                    let mut first_version = chunk.manifest.first_version;
                    let mut last_version = chunk.manifest.last_version;
                    let (
                        mut txns,
                        mut persisted_aux_info,
                        mut txn_infos,
                        mut event_vecs,
                        mut write_sets,
                    ) = chunk.unpack();

                    // remove the txns that exceeds the target_version to be restored
                    if target_version < last_version {
                        let num_to_keep = (target_version - first_version + 1) as usize;
                        txns.drain(num_to_keep..);
                        persisted_aux_info.drain(num_to_keep..);
                        txn_infos.drain(num_to_keep..);
                        event_vecs.drain(num_to_keep..);
                        write_sets.drain(num_to_keep..);
                        last_version = target_version;
                    }

                    // remove the txns that are before the global_first_version
                    if global_first_version > first_version {
                        let num_to_remove = (global_first_version - first_version) as usize;

                        txns.drain(..num_to_remove);
                        persisted_aux_info.drain(..num_to_remove);
                        txn_infos.drain(..num_to_remove);
                        event_vecs.drain(..num_to_remove);
                        write_sets.drain(..num_to_remove);
                        first_version = global_first_version;
                    }

                    // identify txns to be saved before the first_to_replay version
                    if first_version < first_to_replay {
                        let num_to_save =
                            (min(first_to_replay, last_version + 1) - first_version) as usize;
                        let txns_to_save: Vec<_> = txns.drain(..num_to_save).collect();
                        let persisted_aux_info_to_save: Vec<_> =
                            persisted_aux_info.drain(..num_to_save).collect();
                        let txn_infos_to_save: Vec<_> = txn_infos.drain(..num_to_save).collect();
                        let event_vecs_to_save: Vec<_> = event_vecs.drain(..num_to_save).collect();
                        let write_sets_to_save = write_sets.drain(..num_to_save).collect();
                        tokio::task::spawn_blocking(move || {
                            restore_handler.save_transactions(
                                first_version,
                                &txns_to_save,
                                &persisted_aux_info_to_save,
                                &txn_infos_to_save,
                                &event_vecs_to_save,
                                write_sets_to_save,
                            )
                        })
                        .await??;
                        let last_saved = first_version + num_to_save as u64 - 1;
                        TRANSACTION_SAVE_VERSION.set(last_saved as i64);
                        info!(
                            version = last_saved,
                            accumulative_tps = ((last_saved - global_first_version + 1) as f64
                                / start.elapsed().as_secs_f64())
                                as u64,
                            "Transactions saved."
                        );
                    }

                    // create iterator of txn and its outputs to be replayed after the snapshot.
                    Ok(stream::iter(
                        izip!(txns, persisted_aux_info, txn_infos, write_sets, event_vecs)
                            .map(Result::<_>::Ok),
                    ))
                })
            })
            .try_buffered_x(self.global_opt.concurrent_downloads, 1)
            .try_flatten()
            .peekable();

        // Finish saving transactions that are not to be replayed.
        let first_txn_to_replay = {
            Pin::new(&mut txns_to_execute_stream)
                .peek()
                .await
                .map(|res| res.as_ref().map_err(|e| anyhow!("Error: {}", e)))
                .transpose()?
                .map(|_| ())
        };

        Ok(first_txn_to_replay.map(|_| txns_to_execute_stream))
    }

    // only apply KV to the DB
    async fn replay_kv(
        &self,
        restore_handler: &RestoreHandler,
        txns_to_execute_stream: impl Stream<
            Item = Result<(
                Transaction,
                PersistedAuxiliaryInfo,
                TransactionInfo,
                WriteSet,
                Vec<ContractEvent>,
            )>,
        >,
    ) -> Result<()> {
        let (first_version, _) = self.replay_from_version.unwrap();

        let mut base_version = first_version;
        let mut offset = 0u64;
        let replay_start = Instant::now();
        let arc_restore_handler = Arc::new(restore_handler.clone());

        let db_commit_stream = batch_by_blocks(txns_to_execute_stream)
            .map_ok(|chunk| {
                let (txns, persisted_aux_info, txn_infos, write_sets, events): (
                    Vec<_>,
                    Vec<_>,
                    Vec<_>,
                    Vec<_>,
                    Vec<_>,
                ) = chunk.into_iter().multiunzip();
                let handler = arc_restore_handler.clone();
                base_version += offset;
                offset = txns.len() as u64;
                async move {
                    let _timer = OTHER_TIMERS_SECONDS.timer_with(&["replay_txn_chunk_kv_only"]);
                    tokio::task::spawn_blocking(move || {
                        // we directly save transaction and kvs to DB without involving chunk executor
                        handler.save_transactions_and_replay_kv(
                            base_version,
                            &txns,
                            &persisted_aux_info,
                            &txn_infos,
                            &events,
                            write_sets,
                        )?;
                        // return the last version after the replaying
                        Ok(base_version + offset - 1)
                    })
                    .err_into::<anyhow::Error>()
                    .await
                }
            })
            .try_buffered_x(self.global_opt.concurrent_downloads, 1)
            .and_then(future::ready);

        let total_replayed = db_commit_stream
            .and_then(|version| async move {
                let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_txn_chunk_kv_only"]);
                tokio::task::spawn_blocking(move || {
                    // version is the latest version finishing the KV replaying
                    let total_replayed = version - first_version;
                    TRANSACTION_REPLAY_VERSION.set(version as i64);
                    info!(
                        version = version,
                        accumulative_tps =
                            (total_replayed as f64 / replay_start.elapsed().as_secs_f64()) as u64,
                        "KV replayed."
                    );
                    Ok(version)
                })
                .await?
            })
            .try_fold(0, |_total, total| future::ok(total))
            .await?;
        info!(
            total_replayed = total_replayed,
            accumulative_tps =
                (total_replayed as f64 / replay_start.elapsed().as_secs_f64()) as u64,
            "KV Replay finished."
        );
        Ok(())
    }

    // replay the txn to the DB
    async fn replay_transactions(
        &self,
        restore_handler: &RestoreHandler,
        txns_to_execute_stream: impl Stream<
            Item = Result<(
                Transaction,
                PersistedAuxiliaryInfo,
                TransactionInfo,
                WriteSet,
                Vec<ContractEvent>,
            )>,
        >,
    ) -> Result<()> {
        let (first_version, _) = self.replay_from_version.unwrap();
        restore_handler.reset_state_store()?;
        let replay_start = Instant::now();
        let db = DbReaderWriter::from_arc(Arc::clone(&restore_handler.aptosdb));
        let chunk_replayer = Arc::new(ChunkExecutor::<AptosVMBlockExecutor>::new(db));
        let ledger_update_stream = batch_by_blocks(txns_to_execute_stream)
            .map_ok(|chunk| {
                let (txns, persisted_aux_info, txn_infos, write_sets, events): (
                    Vec<_>,
                    Vec<_>,
                    Vec<_>,
                    Vec<_>,
                    Vec<_>,
                ) = chunk.into_iter().multiunzip();
                let chunk_replayer = chunk_replayer.clone();
                let verify_execution_mode = self.verify_execution_mode.clone();

                async move {
                    let _timer = OTHER_TIMERS_SECONDS.timer_with(&["enqueue_chunks"]);

                    tokio::task::spawn_blocking(move || {
                        chunk_replayer.enqueue_chunks(
                            txns,
                            persisted_aux_info,
                            txn_infos,
                            write_sets,
                            events,
                            &verify_execution_mode,
                        )
                    })
                    .await
                    .expect("spawn_blocking failed")
                }
            })
            .try_buffered_x(3, 1)
            .map_ok(|chunks_enqueued| {
                futures::stream::repeat_with(|| Result::Ok(())).take(chunks_enqueued)
            })
            .try_flatten();

        let db_commit_stream = ledger_update_stream
            .map_ok(|()| {
                let chunk_replayer = chunk_replayer.clone();
                async move {
                    let _timer = OTHER_TIMERS_SECONDS.timer_with(&["ledger_update"]);

                    tokio::task::spawn_blocking(move || chunk_replayer.update_ledger())
                        .await
                        .expect("spawn_blocking failed")
                }
            })
            .try_buffered_x(3, 1);

        let total_replayed = db_commit_stream
            .and_then(|()| {
                let chunk_replayer = chunk_replayer.clone();
                async move {
                    let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit"]);

                    tokio::task::spawn_blocking(move || {
                        let v = chunk_replayer.commit()?;

                        let total_replayed = v + 1 - first_version;
                        TRANSACTION_REPLAY_VERSION.set(v as i64);
                        info!(
                            version = v,
                            accumulative_tps = (total_replayed as f64
                                / replay_start.elapsed().as_secs_f64())
                                as u64,
                            "Transactions replayed."
                        );
                        Ok(total_replayed)
                    })
                    .await
                    .expect("spawn_blocking failed")
                }
            })
            .try_fold(0, |_prev_total, total| future::ok(total))
            .await?;

        // assert all chunks are fully processed and in DB.
        assert!(chunk_replayer.is_empty());

        info!(
            total_replayed = total_replayed,
            accumulative_tps =
                (total_replayed as f64 / replay_start.elapsed().as_secs_f64()) as u64,
            "Replay finished."
        );
        Ok(())
    }

    async fn go_through_verified_chunks(
        &self,
        loaded_chunk_stream: impl Stream<Item = Result<LoadedChunk>>,
        first_version: Version,
    ) -> Result<()> {
        let analysis = self
            .output_transaction_analysis
            .as_ref()
            .map(|dir| TransactionAnalysis::new(dir))
            .transpose()?;
        let start = Instant::now();
        loaded_chunk_stream
            .try_fold(analysis, |mut analysis, chunk| async move {
                let last_version = chunk.manifest.last_version;

                for (version, (txn, persisted_aux_info, txn_info, events, write_set)) in
                    (chunk.manifest.first_version..).zip(itertools::multizip(chunk.unpack()))
                {
                    if let Some(analysis) = &mut analysis {
                        analysis.add_transaction(
                            version,
                            &txn,
                            &persisted_aux_info,
                            &txn_info,
                            &events,
                            &write_set,
                        )?;
                    }
                }

                VERIFY_TRANSACTION_VERSION.set(last_version as i64);
                info!(
                    version = last_version,
                    accumulative_tps = ((last_version - first_version + 1) as f64
                        / start.elapsed().as_secs_f64())
                        as u64,
                    "Transactions verified."
                );
                Ok(analysis)
            })
            .await?;
        Ok(())
    }
}

type TxnToReplay = (
    Transaction,
    PersistedAuxiliaryInfo,
    TransactionInfo,
    WriteSet,
    Vec<ContractEvent>,
);

/// Groups txns to replay into batches with `BlockBatcher`.
fn batch_by_blocks(
    txns: impl Stream<Item = Result<TxnToReplay>>,
) -> impl Stream<Item = Result<Vec<TxnToReplay>>> {
    txns.map_ok(Some)
        .chain(stream::once(future::ok(None)))
        .scan(BlockBatcher::new(BATCH_SIZE), |batcher, txn| {
            future::ready(Some(txn.map(|txn| match txn {
                Some(txn) => {
                    let (_, _, txn_info, _, events) = &txn;
                    let ends_block = txn_info.has_state_checkpoint_hash();
                    let ends_epoch = events.iter().any(ContractEvent::is_new_epoch_event);
                    batcher.push(txn, ends_block, ends_epoch)
                },
                None => batcher.finish(),
            })))
        })
        .try_filter_map(future::ok)
}

/// Groups txns to replay into batches of whole blocks. KV replay persists `VersionData` at batch
/// ends, and an interrupted restore resumes after the last batch committed. Full replay verifies
/// each batch by re-executing it, which should start at the beginning of a block. A batch ends at
/// the first block end once it has `min_size` txns, or at an epoch end, since the first txn of the
/// next epoch reads `VersionData` via `state_storage::on_new_block`.
struct BlockBatcher<T> {
    min_size: usize,
    batch: Vec<T>,
}

impl<T> BlockBatcher<T> {
    fn new(min_size: usize) -> Self {
        Self {
            min_size,
            batch: Vec::new(),
        }
    }

    /// Returns the batch `txn` completes, if any.
    fn push(&mut self, txn: T, ends_block: bool, ends_epoch: bool) -> Option<Vec<T>> {
        self.batch.push(txn);
        (ends_epoch || ends_block && self.batch.len() >= self.min_size)
            .then(|| std::mem::take(&mut self.batch))
    }

    /// Returns what's left at the end of the input.
    fn finish(&mut self) -> Option<Vec<T>> {
        (!self.batch.is_empty()).then(|| std::mem::take(&mut self.batch))
    }
}

#[cfg(test)]
mod tests {
    use super::{BlockBatcher, TransactionRestoreBatchController, BATCH_SIZE};
    use crate::{
        backup_types::transaction::backup::{TransactionBackupController, TransactionBackupOpt},
        storage::{local_fs::LocalFs, BackupStorage},
        utils::{
            backup_service_client::BackupServiceClient, test_utils::start_local_backup_service,
            GlobalBackupOpt, GlobalRestoreOptions, RestoreRunMode,
        },
    };
    use aptos_db::{get_restore_handler::GetRestoreHandler, AptosDB};
    use aptos_executor_test_helpers::integration_test_impl::{
        create_db_and_executor, test_execution_with_storage_impl_inner,
    };
    use aptos_executor_types::VerifyExecutionMode;
    use aptos_storage_interface::DbReader;
    use aptos_temppath::TempPath;
    use aptos_types::transaction::Version;
    use futures::stream;
    use itertools::{izip, Itertools};
    use std::{collections::HashSet, sync::Arc, time::Duration};

    #[test]
    fn test_restore_ends_at_block_end() {
        let source_path = TempPath::new();
        let source = test_execution_with_storage_impl_inner(false, source_path.path());
        let source_version = source.expect_synced_version();
        let block_ends: Vec<_> = source
            .get_transaction_info_iterator(0, source_version + 1)
            .unwrap()
            .positions(|info| info.unwrap().has_state_checkpoint_hash())
            .map(|idx| idx as Version)
            .collect();
        // A target in the middle of the second block after genesis.
        let last_block_end = block_ends[1];
        let target_version = block_ends[2] - 1;
        assert!(target_version > last_block_end);

        let (rt, port) = start_local_backup_service(Arc::clone(&source));
        let client = Arc::new(BackupServiceClient::new(format!(
            "http://localhost:{}",
            port
        )));
        let backup_path = TempPath::new();
        backup_path.create_as_dir().unwrap();
        let store: Arc<dyn BackupStorage> =
            Arc::new(LocalFs::new(backup_path.path().to_path_buf()));
        let manifest = rt
            .block_on(
                TransactionBackupController::new(
                    TransactionBackupOpt {
                        start_version: 0,
                        num_transactions: source_version as usize + 1,
                    },
                    // A txn per chunk, so the block end is searched for across chunks.
                    GlobalBackupOpt {
                        max_chunk_size: 1,
                        concurrent_data_requests: 2,
                    },
                    client,
                    Arc::clone(&store),
                )
                .run(),
            )
            .unwrap();

        // No replay, full replay and KV only replay.
        for replay in [None, Some(false), Some(true)] {
            let target_path = TempPath::new();
            let target = Arc::new(AptosDB::new_for_test(target_path.path()));
            let restore = |first_version| {
                rt.block_on(
                    TransactionRestoreBatchController::new(
                        GlobalRestoreOptions {
                            target_version,
                            trusted_waypoints: Arc::new(Default::default()),
                            run_mode: Arc::new(RestoreRunMode::Restore {
                                restore_handler: target.get_restore_handler(),
                            }),
                            concurrent_downloads: 1,
                            replay_concurrency_level: 1,
                        },
                        Arc::clone(&store),
                        vec![manifest.clone()],
                        first_version,
                        replay.map(|kv_only| (first_version.unwrap_or(0), kv_only)),
                        None,
                        VerifyExecutionMode::verify_all(),
                        None,
                    )
                    .run(),
                )
            };

            restore(None).unwrap();
            assert_eq!(target.expect_synced_version(), last_block_end);
            assert_eq!(
                target.get_accumulator_root_hash(last_block_end).unwrap(),
                source.get_accumulator_root_hash(last_block_end).unwrap(),
            );

            // A rerun finds the DB at the block end already. With a txn per chunk, no chunk is left
            // to restore past it.
            restore(Some(last_block_end + 1)).unwrap();
            assert_eq!(target.expect_synced_version(), last_block_end);
        }

        rt.shutdown_timeout(Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_replay_kv() {
        let source_path = TempPath::new();
        let source = test_execution_with_storage_impl_inner(false, source_path.path());
        let target_version = source.expect_synced_version();
        let (outputs, persisted_aux_info) = source
            .get_transaction_outputs(0, target_version + 1, target_version)
            .unwrap()
            .into_parts();
        let transactions: Vec<_> = izip!(
            outputs.transactions_and_outputs,
            persisted_aux_info,
            outputs.proof.transaction_infos,
        )
        .map(|((transaction, output), aux_info, info)| {
            (
                transaction,
                aux_info,
                info,
                output.write_set().clone(),
                output.events().to_vec(),
            )
        })
        .collect();
        let replayed_keys: HashSet<_> = transactions[1..]
            .iter()
            .flat_map(|(.., write_set, _)| write_set.write_op_iter().map(|(key, _)| key.clone()))
            .collect();
        // Multiple blocks, which batching by `BATCH_SIZE` alone would split.
        let block_ends: Vec<_> = transactions[1..]
            .iter()
            .positions(|(_, _, info, ..)| info.has_state_checkpoint_hash())
            .collect();
        assert!(block_ends.len() > 1);
        assert!(block_ends[0] + 1 > BATCH_SIZE);

        // KV replay needs the state right before the first replayed version in the DB.
        let target_path = TempPath::new();
        let (target, ..) = create_db_and_executor(target_path.path(), &transactions[0].0, false);
        let restore_handler = target.get_restore_handler();
        let backup_path = TempPath::new();
        let controller = TransactionRestoreBatchController::new(
            GlobalRestoreOptions {
                target_version,
                trusted_waypoints: Arc::new(Default::default()),
                run_mode: Arc::new(RestoreRunMode::Restore {
                    restore_handler: restore_handler.clone(),
                }),
                concurrent_downloads: 1,
                replay_concurrency_level: 1,
            },
            Arc::new(LocalFs::new(backup_path.path().to_path_buf())),
            vec![],
            None,
            Some((1, true)),
            None,
            VerifyExecutionMode::NoVerify,
            None,
        );
        controller
            .replay_kv(
                &restore_handler,
                stream::iter(transactions.into_iter().skip(1).map(Ok)),
            )
            .await
            .unwrap();

        for key in &replayed_keys {
            assert_eq!(
                target
                    .get_state_value_by_version(key, target_version)
                    .unwrap(),
                source
                    .get_state_value_by_version(key, target_version)
                    .unwrap(),
            );
        }
        // Usage only matches if each chunk saw the values committed by previous chunks. The source
        // has usage at every block end and nowhere else, so the target only matches it if every
        // chunk is exactly one whole block.
        for version in 0..=target_version {
            assert_eq!(
                target.get_state_storage_usage(Some(version)).ok(),
                source.get_state_storage_usage(Some(version)).ok(),
                "version {version}",
            );
        }
    }

    #[test]
    fn test_block_batcher() {
        // `|` marks a block end, `!` an epoch end, which is also a block end.
        let batch = |txns: &str, min_size| {
            let mut batcher = BlockBatcher::new(min_size);
            let mut batches: Vec<String> = txns
                .chars()
                .filter_map(|c| batcher.push(c, matches!(c, '|' | '!'), c == '!'))
                .map(String::from_iter)
                .collect();
            batches.extend(batcher.finish().map(String::from_iter));
            batches
        };

        assert!(batch("", 2).is_empty());
        assert_eq!(batch("a|bc|", 1), vec!["a|", "bc|"]);
        // A batch extends to the first block end once it has the min size.
        assert_eq!(batch("a|b|c|", 3), vec!["a|b|", "c|"]);
        assert_eq!(batch("abc|de|", 2), vec!["abc|", "de|"]);
        // An epoch end always ends a batch.
        assert_eq!(batch("a!b|c|", 4), vec!["a!", "b|c|"]);
        assert_eq!(batch("!!", 4), vec!["!", "!"]);
        // What's left at the end of the input is flushed.
        assert_eq!(batch("ab|cd", 2), vec!["ab|", "cd"]);
        assert_eq!(batch("ab", 1), vec!["ab"]);
    }
}
