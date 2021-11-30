// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup_types::{
        epoch_ending::restore::EpochHistory,
        transaction::manifest::{TransactionBackup, TransactionChunk},
    },
    metrics::{
        restore::{TRANSACTION_REPLAY_VERSION, TRANSACTION_SAVE_VERSION},
        verify::VERIFY_TRANSACTION_VERSION,
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
use diem_logger::prelude::*;
use diem_types::{
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    proof::{TransactionAccumulatorRangeProof, TransactionInfoListWithProof},
    transaction::{
        default_protocol::TransactionListWithProof, Transaction, TransactionInfo, Version,
    },
};
use diem_vm::DiemVM;
use diemdb::backup::restore_handler::RestoreHandler;
use executor::chunk_executor::ChunkExecutor;
use executor_types::TransactionReplayer;
use futures::{
    future,
    future::TryFutureExt,
    stream,
    stream::{Peekable, Stream, TryStreamExt},
    StreamExt,
};
use itertools::zip_eq;
use std::{cmp::min, pin::Pin, sync::Arc, time::Instant};
use storage_interface::DbReaderWriter;
use structopt::StructOpt;
use tokio::io::BufReader;

#[cfg(not(test))]
const BATCH_SIZE: usize = 10000;
#[cfg(test)]
const BATCH_SIZE: usize = 2;

#[derive(StructOpt)]
pub struct TransactionRestoreOpt {
    #[structopt(long = "transaction-manifest")]
    pub manifest_handle: FileHandle,
    #[structopt(
        long = "replay-transactions-from-version",
        help = "Transactions with this version and above will be replayed so state and events are \
        gonna pop up. Requires state at the version right before this to exist, either by \
        recovering a state snapshot, or previous transaction replay."
    )]
    pub replay_from_version: Option<Version>,
}

impl TransactionRestoreOpt {
    pub fn replay_from_version(&self) -> Version {
        self.replay_from_version.unwrap_or(Version::max_value())
    }
}

pub struct TransactionRestoreController {
    inner: TransactionRestoreBatchController,
}

#[allow(dead_code)]
struct LoadedChunk {
    pub manifest: TransactionChunk,
    pub txns: Vec<Transaction>,
    pub txn_infos: Vec<TransactionInfo>,
    pub event_vecs: Vec<Vec<ContractEvent>>,
    pub range_proof: TransactionAccumulatorRangeProof,
    pub ledger_info: LedgerInfoWithSignatures,
}

impl LoadedChunk {
    async fn load(
        manifest: TransactionChunk,
        storage: &Arc<dyn BackupStorage>,
        epoch_history: Option<&Arc<EpochHistory>>,
    ) -> Result<Self> {
        let mut file = BufReader::new(storage.open_for_read(&manifest.transactions).await?);
        let mut txns = Vec::new();
        let mut txn_infos = Vec::new();
        let mut event_vecs = Vec::new();

        while let Some(record_bytes) = file.read_record_bytes().await? {
            let (txn, txn_info, events) = bcs::from_bytes(&record_bytes)?;
            txns.push(txn);
            txn_infos.push(txn_info);
            event_vecs.push(events);
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
        let txn_list_with_proof = TransactionListWithProof::new(
            txns,
            Some(event_vecs),
            Some(manifest.first_version),
            TransactionInfoListWithProof::new(range_proof, txn_infos),
        );
        txn_list_with_proof.verify(ledger_info.ledger_info(), Some(manifest.first_version))?;
        // and disassemble it to get things back.
        let txns = txn_list_with_proof.transactions;
        let range_proof = txn_list_with_proof
            .proof
            .ledger_info_to_transaction_infos_proof;
        let txn_infos = txn_list_with_proof.proof.transaction_infos;
        let event_vecs = txn_list_with_proof.events.expect("unknown to be Some.");

        Ok(Self {
            manifest,
            txns,
            txn_infos,
            event_vecs,
            range_proof,
            ledger_info,
        })
    }
}

impl TransactionRestoreController {
    pub fn new(
        opt: TransactionRestoreOpt,
        global_opt: GlobalRestoreOptions,
        storage: Arc<dyn BackupStorage>,
        epoch_history: Option<Arc<EpochHistory>>,
    ) -> Self {
        let inner = TransactionRestoreBatchController::new(
            global_opt,
            storage,
            vec![opt.manifest_handle],
            opt.replay_from_version,
            epoch_history,
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
    replay_from_version: Option<Version>,
    epoch_history: Option<Arc<EpochHistory>>,
}

impl TransactionRestoreBatchController {
    pub fn new(
        global_opt: GlobalRestoreOptions,
        storage: Arc<dyn BackupStorage>,
        manifest_handles: Vec<FileHandle>,
        replay_from_version: Option<Version>,
        epoch_history: Option<Arc<EpochHistory>>,
    ) -> Self {
        Self {
            global_opt,
            storage,
            manifest_handles,
            replay_from_version,
            epoch_history,
        }
    }

    pub async fn run(self) -> Result<()> {
        let name = self.name();
        info!("{} started.", name);
        let res = self
            .run_impl()
            .await
            .map_err(|e| anyhow!("{} failed: {}", name, e))?;
        info!("{} succeeded.", name);
        Ok(res)
    }

    fn name(&self) -> String {
        format!("transaction {}", self.global_opt.run_mode.name())
    }

    async fn run_impl(self) -> Result<()> {
        if self.manifest_handles.is_empty() {
            return Ok(());
        }

        let mut loaded_chunk_stream = self.loaded_chunk_stream();
        let first_version = self
            .confirm_or_save_frozen_subtrees(&mut loaded_chunk_stream)
            .await?;

        if let RestoreRunMode::Restore { restore_handler } = self.global_opt.run_mode.as_ref() {
            let txns_to_execute_stream = self
                .save_before_replay_version(first_version, loaded_chunk_stream, restore_handler)
                .await?;

            if let Some(txns_to_execute_stream) = txns_to_execute_stream {
                self.replay_transactions(restore_handler, txns_to_execute_stream)
                    .await?;
            }
        } else {
            Self::go_through_verified_chunks(loaded_chunk_stream, first_version).await?;
        }
        Ok(())
    }

    fn loaded_chunk_stream(&self) -> Peekable<impl Stream<Item = Result<LoadedChunk>>> {
        let con = self.global_opt.concurrent_downloads;

        let manifest_handle_stream = stream::iter(self.manifest_handles.clone().into_iter());

        let storage = self.storage.clone();
        let manifest_stream = manifest_handle_stream
            .map(move |hdl| {
                let storage = storage.clone();
                async move { storage.load_json_file(&hdl).await.err_notes(&hdl) }
            })
            .buffered_x(con * 3, con)
            .and_then(|m: TransactionBackup| future::ready(m.verify().map(|_| m)));

        let target_version = self.global_opt.target_version;
        let chunk_manifest_stream = manifest_stream
            .map_ok(|m| stream::iter(m.chunks.into_iter().map(Result::<_>::Ok)))
            .try_flatten()
            .try_take_while(move |c| future::ready(Ok(c.first_version <= target_version)))
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
                    }
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
    ) -> Result<Option<impl Stream<Item = Result<(Transaction, TransactionInfo)>>>> {
        let start = Instant::now();

        let restore_handler_clone = restore_handler.clone();
        let first_to_replay = self.replay_from_version.unwrap_or(Version::MAX);
        let target_version = self.global_opt.target_version;

        let mut txns_to_execute_stream = loaded_chunk_stream
            .and_then(move |chunk| {
                let restore_handler = restore_handler_clone.clone();
                future::ok(async move {
                    let LoadedChunk {
                        manifest:
                            TransactionChunk {
                                first_version,
                                mut last_version,
                                transactions: _,
                                proof: _,
                            },
                        mut txns,
                        mut txn_infos,
                        mut event_vecs,
                        range_proof: _,
                        ledger_info: _,
                    } = chunk;

                    if target_version < last_version {
                        let num_to_keep = (target_version - first_version + 1) as usize;
                        txns.drain(num_to_keep..);
                        txn_infos.drain(num_to_keep..);
                        event_vecs.drain(num_to_keep..);
                        last_version = target_version;
                    }

                    if first_version < first_to_replay {
                        let num_to_save =
                            (min(first_to_replay, last_version + 1) - first_version) as usize;
                        let txns_to_save: Vec<_> = txns.drain(..num_to_save).collect();
                        let txn_infos_to_save: Vec<_> = txn_infos.drain(..num_to_save).collect();
                        let event_vecs_to_save: Vec<_> = event_vecs.drain(..num_to_save).collect();

                        tokio::task::spawn_blocking(move || {
                            restore_handler.save_transactions(
                                first_version,
                                &txns_to_save,
                                &txn_infos_to_save,
                                &event_vecs_to_save,
                            )
                        })
                        .await??;
                        let last_saved = first_version + num_to_save as u64 - 1;
                        TRANSACTION_SAVE_VERSION.set(last_saved as i64);
                        info!(
                            version = last_saved,
                            accumulative_tps = (last_saved - global_first_version + 1) as f64
                                / start.elapsed().as_secs_f64(),
                            "Transactions saved."
                        );
                    }

                    Ok(stream::iter(
                        zip_eq(txns, txn_infos).into_iter().map(Result::<_>::Ok),
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

    async fn replay_transactions(
        &self,
        restore_handler: &RestoreHandler,
        txns_to_execute_stream: impl Stream<Item = Result<(Transaction, TransactionInfo)>>,
    ) -> Result<()> {
        let replay_start = Instant::now();
        let first_version = self.replay_from_version.unwrap();
        let db = DbReaderWriter::from_arc(Arc::clone(&restore_handler.diemdb));
        let persisted_view = restore_handler.get_tree_state(first_version)?.into();
        let chunk_replayer = Arc::new(ChunkExecutor::<DiemVM>::new_with_view(db, persisted_view));

        let db_commit_stream = txns_to_execute_stream
            .try_chunks(BATCH_SIZE)
            .err_into::<anyhow::Error>()
            .map_ok(|chunk| {
                let (txns, txn_infos): (Vec<_>, Vec<_>) = chunk.into_iter().unzip();
                let chunk_replayer = chunk_replayer.clone();
                async move {
                    tokio::task::spawn_blocking(move || chunk_replayer.replay(txns, txn_infos))
                        .err_into::<anyhow::Error>()
                        .await
                }
            })
            .try_buffered_x(self.global_opt.concurrent_downloads, 1)
            .and_then(future::ready);

        db_commit_stream
            .and_then(|()| {
                let chunk_replayer = chunk_replayer.clone();
                async move {
                    tokio::task::spawn_blocking(move || {
                        let committed_chunk = chunk_replayer.commit()?;
                        let v = committed_chunk.result_view.version().unwrap_or(0);
                        TRANSACTION_REPLAY_VERSION.set(v as i64);
                        info!(
                            version = v,
                            accumulative_tps = (v - first_version + 1) as f64
                                / replay_start.elapsed().as_secs_f64(),
                            "Transactions replayed."
                        );
                        Ok(())
                    })
                    .await?
                }
            })
            .try_fold((), |(), ()| future::ok(()))
            .await
    }

    async fn go_through_verified_chunks(
        loaded_chunk_stream: impl Stream<Item = Result<LoadedChunk>>,
        first_version: Version,
    ) -> Result<()> {
        let start = Instant::now();
        loaded_chunk_stream
            .try_fold((), |(), chunk| {
                let v = chunk.manifest.last_version;
                VERIFY_TRANSACTION_VERSION.set(v as i64);
                info!(
                    version = v,
                    accumulative_tps =
                        (v - first_version + 1) as f64 / start.elapsed().as_secs_f64(),
                    "Transactions verified."
                );
                future::ok(())
            })
            .await
    }
}
