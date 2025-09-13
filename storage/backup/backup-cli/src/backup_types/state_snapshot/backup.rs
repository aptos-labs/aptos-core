// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup_types::state_snapshot::manifest::{StateSnapshotBackup, StateSnapshotChunk},
    metadata::Metadata,
    metrics::backup::BACKUP_TIMER,
    storage::{BackupHandleRef, BackupStorage, FileHandle, ShellSafeName},
    utils::{
        backup_service_client::BackupServiceClient, read_record_bytes::ReadRecordBytes,
        should_cut_chunk, storage_ext::BackupStorageExt, stream::TryStreamX, GlobalBackupOpt,
    },
};
use anyhow::{anyhow, ensure, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_logger::prelude::*;
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    proof::TransactionInfoWithProof,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use bytes::{BufMut, Bytes, BytesMut};
use clap::Parser;
use futures::{StreamExt, TryStream, TryStreamExt};
use once_cell::sync::Lazy;
use std::{convert::TryInto, str::FromStr, sync::Arc, time::Instant};
use tokio::{io::AsyncWriteExt, sync::mpsc::Sender};
use tokio_stream::wrappers::ReceiverStream;

#[derive(Parser)]
pub struct StateSnapshotBackupOpt {
    #[clap(
        long = "state-snapshot-epoch",
        help = "Epoch at the end of which a state snapshot is to be taken."
    )]
    pub epoch: u64,
}

struct Chunk {
    bytes: Bytes,
    first_key: HashValue,
    first_idx: usize,
    last_key: HashValue,
    last_idx: usize,
}

struct ChunkerState<RecordStream> {
    record_stream: Option<RecordStream>,
    buf: BytesMut,
    chunk_first_key: HashValue,
    prev_record_len: usize,
    current_idx: usize,
    chunk_first_idx: usize,
    max_chunk_size: usize,
}

impl<RecordStream> ChunkerState<RecordStream>
where
    RecordStream: TryStream<Ok = Bytes, Error = anyhow::Error> + Unpin,
{
    async fn new(mut record_stream: RecordStream, max_chunk_size: usize) -> Result<Self> {
        let first_record = record_stream
            .try_next()
            .await?
            .ok_or_else(|| anyhow!("State is empty."))?;

        let chunk_first_key = Self::parse_key(&first_record)?;
        let prev_record_len = first_record.len();

        let mut buf = BytesMut::new();
        buf.put_slice(&(first_record.len() as u32).to_be_bytes());
        buf.extend(first_record);

        Ok(Self {
            record_stream: Some(record_stream),
            buf,
            chunk_first_key,
            prev_record_len,
            current_idx: 0,
            chunk_first_idx: 0,
            max_chunk_size,
        })
    }

    async fn next_full_chunk(&mut self) -> Result<Option<Chunk>> {
        let _timer = BACKUP_TIMER.timer_with(&["state_snapshot_next_full_chunk"]);

        let input = self
            .record_stream
            .as_mut()
            .expect("get_next_full_chunk after EOF.");

        while let Some(record_bytes) = input.try_next().await? {
            let _timer = BACKUP_TIMER.timer_with(&["state_snapshot_process_records"]);

            // If buf + current_record exceeds max_chunk_size, dump current buf to a new chunk
            let chunk_cut_opt = should_cut_chunk(&self.buf, &record_bytes, self.max_chunk_size)
                .then(|| {
                    let bytes = self.buf.split().freeze();
                    let last_key = Self::parse_key(&bytes[bytes.len() - self.prev_record_len..])?;

                    let chunk = Chunk {
                        bytes,
                        first_key: self.chunk_first_key,
                        first_idx: self.chunk_first_idx,
                        last_key,
                        last_idx: self.current_idx,
                    };

                    self.chunk_first_idx = self.current_idx + 1;
                    self.chunk_first_key = Self::parse_key(&record_bytes)?;

                    Result::<_>::Ok(chunk)
                })
                .transpose()?;

            // Append record to buf
            self.prev_record_len = record_bytes.len();
            self.buf
                .put_slice(&(record_bytes.len() as u32).to_be_bytes());
            self.buf.extend(record_bytes);
            self.current_idx += 1;

            // Return the full chunk if found
            if let Some(chunk) = chunk_cut_opt {
                return Ok(Some(chunk));
            }
        }

        // Input file ended, full chunk not found.
        // The call site will call get_last_chunk which consume ChunkerState
        let _ = self.record_stream.take();
        Ok(None)
    }

    async fn last_chunk(self) -> Result<Chunk> {
        let Self {
            record_stream: state_snapshot_file,
            buf,
            chunk_first_key,
            prev_record_len,
            current_idx,
            chunk_first_idx,
            max_chunk_size: _,
        } = self;
        ensure!(
            state_snapshot_file.is_none(),
            "get_last_chunk called before EOF"
        );
        ensure!(!buf.is_empty(), "Last chunk can't be empty");

        let bytes = buf.freeze();
        let last_key = Self::parse_key(&bytes[bytes.len() - prev_record_len..])?;

        Ok(Chunk {
            bytes,
            first_key: chunk_first_key,
            first_idx: chunk_first_idx,
            last_key,
            last_idx: current_idx,
        })
    }

    fn parse_key(record: &[u8]) -> Result<HashValue> {
        let (key, _): (StateKey, StateValue) = bcs::from_bytes(record)?;
        Ok(key.hash())
    }
}

struct Chunker<R> {
    state: Option<ChunkerState<R>>,
}

impl<RecordStream> Chunker<RecordStream>
where
    RecordStream: TryStream<Ok = Bytes, Error = anyhow::Error> + Unpin,
{
    async fn new(record_stream: RecordStream, max_chunk_size: usize) -> Result<Self> {
        Ok(Self {
            state: Some(ChunkerState::new(record_stream, max_chunk_size).await?),
        })
    }

    async fn next_chunk(&mut self) -> Result<Option<Chunk>> {
        let ret = match self.state.as_mut() {
            None => None,
            Some(state) => match state.next_full_chunk().await? {
                Some(chunk) => Some(chunk),
                None => Some(self.state.take().unwrap().last_chunk().await?),
            },
        };

        Ok(ret)
    }
}

pub struct StateSnapshotBackupController {
    epoch: u64,
    version: Option<Version>, // initialize before using
    max_chunk_size: usize,
    client: Arc<BackupServiceClient>,
    storage: Arc<dyn BackupStorage>,
    concurrent_data_requests: usize,
}

impl StateSnapshotBackupController {
    pub fn new(
        opt: StateSnapshotBackupOpt,
        global_opt: GlobalBackupOpt,
        client: Arc<BackupServiceClient>,
        storage: Arc<dyn BackupStorage>,
    ) -> Self {
        Self {
            epoch: opt.epoch,
            version: None,
            max_chunk_size: global_opt.max_chunk_size,
            client,
            storage,
            concurrent_data_requests: global_opt.concurrent_data_requests,
        }
    }

    pub async fn run(self) -> Result<FileHandle> {
        info!("State snapshot backup started, for epoch {}.", self.epoch);
        let ret = self
            .run_impl()
            .await
            .map_err(|e| anyhow!("State snapshot backup failed: {}", e))?;
        info!("State snapshot backup succeeded. Manifest: {}", ret);
        Ok(ret)
    }

    async fn run_impl(mut self) -> Result<FileHandle> {
        self.version = Some(self.get_version_for_epoch_ending(self.epoch).await?);
        let backup_handle = self
            .storage
            .create_backup_with_random_suffix(&self.backup_name())
            .await?;

        let record_stream = Box::pin(self.record_stream(self.concurrent_data_requests).await?);
        let chunker = Chunker::new(record_stream, self.max_chunk_size).await?;

        let start = Instant::now();
        let chunk_stream = futures::stream::try_unfold(chunker, |mut chunker| async {
            Ok(chunker.next_chunk().await?.map(|chunk| (chunk, chunker)))
        });

        let chunk_manifest_fut_stream =
            chunk_stream.map_ok(|chunk| self.write_chunk(&backup_handle, chunk));

        let chunks: Vec<_> = chunk_manifest_fut_stream
            .try_buffered_x(8, 4) // 4 concurrently, at most 8 results in buffer.
            .map_ok(|chunk_manifest| {
                let last_idx = chunk_manifest.last_idx;
                info!(
                    last_idx = last_idx,
                    values_per_second =
                        ((last_idx + 1) as f64 / start.elapsed().as_secs_f64()) as u64,
                    "Chunk written."
                );
                chunk_manifest
            })
            .try_collect()
            .await?;

        self.write_manifest(&backup_handle, chunks).await
    }

    async fn record_stream(
        &self,
        concurrency: usize,
    ) -> Result<impl TryStream<Ok = Bytes, Error = anyhow::Error, Item = Result<Bytes>> + use<>>
    {
        const CHUNK_SIZE: usize = if cfg!(test) { 2 } else { 100_000 };

        let count = self.client.get_state_item_count(self.version()).await?;
        let version = self.version();
        let client = self.client.clone();

        let chunks_stream = futures::stream::unfold(0, move |start_idx| async move {
            if start_idx >= count {
                return None;
            }

            let next_start_idx = start_idx + CHUNK_SIZE;
            let chunk_size = CHUNK_SIZE.min(count - start_idx);

            Some(((start_idx, chunk_size), next_start_idx))
        })
        .map(Result::<_>::Ok);

        let record_stream_stream = chunks_stream.map_ok(move |(start_idx, chunk_size)| {
            let client = client.clone();
            async move {
                let (tx, rx) = tokio::sync::mpsc::channel(chunk_size);
                // spawn and forget, propagate error through channel
                let _join_handle = tokio::spawn(send_records(
                    client.clone(),
                    version,
                    start_idx,
                    chunk_size,
                    tx,
                ));

                Ok(ReceiverStream::new(rx))
            }
        });

        Ok(record_stream_stream
            .try_buffered_x(concurrency * 2, concurrency)
            .try_flatten())
    }
}

async fn send_records(
    client: Arc<BackupServiceClient>,
    version: Version,
    start_idx: usize,
    chunk_size: usize,
    sender: Sender<Result<Bytes>>,
) {
    if let Err(err) = send_records_inner(client, version, start_idx, chunk_size, &sender).await {
        let _ = sender.send(Err(err)).await;
    }
}

async fn send_records_inner(
    client: Arc<BackupServiceClient>,
    version: Version,
    start_idx: usize,
    chunk_size: usize,
    sender: &Sender<Result<Bytes>>,
) -> Result<()> {
    let _timer = BACKUP_TIMER.timer_with(&["state_snapshot_record_stream_all"]);
    let mut input = client
        .get_state_snapshot_chunk(version, start_idx, chunk_size)
        .await?;
    let mut count = 0;
    while let Some(record_bytes) = {
        let _timer = BACKUP_TIMER.timer_with(&["state_snapshot_read_record_bytes"]);
        input.read_record_bytes().await?
    } {
        let _timer = BACKUP_TIMER.timer_with(&["state_snapshot_record_stream_send_bytes"]);
        count += 1;
        sender.send(Ok(record_bytes)).await?;
    }
    ensure!(
        count == chunk_size,
        "expecting {} records, got {}",
        chunk_size,
        count
    );
    Ok(())
}

impl StateSnapshotBackupController {
    fn version(&self) -> Version {
        self.version.unwrap()
    }

    fn backup_name(&self) -> String {
        format!("state_epoch_{}_ver_{}", self.epoch, self.version())
    }

    fn manifest_name() -> &'static ShellSafeName {
        static NAME: Lazy<ShellSafeName> =
            Lazy::new(|| ShellSafeName::from_str("state.manifest").unwrap());
        &NAME
    }

    fn proof_name() -> &'static ShellSafeName {
        static NAME: Lazy<ShellSafeName> =
            Lazy::new(|| ShellSafeName::from_str("state.proof").unwrap());
        &NAME
    }

    fn chunk_name(first_idx: usize) -> ShellSafeName {
        format!("{}-.chunk", first_idx).try_into().unwrap()
    }

    fn chunk_proof_name(first_idx: usize, last_idx: usize) -> ShellSafeName {
        format!("{}-{}.proof", first_idx, last_idx)
            .try_into()
            .unwrap()
    }

    async fn get_version_for_epoch_ending(&self, epoch: u64) -> Result<u64> {
        let ledger_info: LedgerInfoWithSignatures = bcs::from_bytes(
            self.client
                .get_epoch_ending_ledger_infos(epoch, epoch + 1)
                .await?
                .read_record_bytes()
                .await?
                .ok_or_else(|| {
                    anyhow!("Failed to get epoch ending ledger info for epoch {}", epoch)
                })?
                .as_ref(),
        )?;
        Ok(ledger_info.ledger_info().version())
    }

    async fn write_chunk(
        &self,
        backup_handle: &BackupHandleRef,
        chunk: Chunk,
    ) -> Result<StateSnapshotChunk> {
        let _timer = BACKUP_TIMER.timer_with(&["state_snapshot_write_chunk"]);

        let Chunk {
            bytes,
            first_idx,
            last_idx,
            first_key,
            last_key,
        } = chunk;

        let (chunk_handle, mut chunk_file) = self
            .storage
            .create_for_write(backup_handle, &Self::chunk_name(first_idx))
            .await?;
        chunk_file.write_all(&bytes).await?;
        chunk_file.shutdown().await?;
        let (proof_handle, mut proof_file) = self
            .storage
            .create_for_write(backup_handle, &Self::chunk_proof_name(first_idx, last_idx))
            .await?;
        tokio::io::copy(
            &mut self
                .client
                .get_account_range_proof(last_key, self.version())
                .await?,
            &mut proof_file,
        )
        .await?;
        proof_file.shutdown().await?;

        Ok(StateSnapshotChunk {
            first_idx,
            last_idx,
            first_key,
            last_key,
            blobs: chunk_handle,
            proof: proof_handle,
        })
    }

    async fn write_manifest(
        &self,
        backup_handle: &BackupHandleRef,
        chunks: Vec<StateSnapshotChunk>,
    ) -> Result<FileHandle> {
        let proof_bytes = self.client.get_state_root_proof(self.version()).await?;
        let (txn_info, _): (TransactionInfoWithProof, LedgerInfoWithSignatures) =
            bcs::from_bytes(&proof_bytes)?;

        let (proof_handle, mut proof_file) = self
            .storage
            .create_for_write(backup_handle, Self::proof_name())
            .await?;
        proof_file.write_all(&proof_bytes).await?;
        proof_file.shutdown().await?;

        let manifest = StateSnapshotBackup {
            epoch: self.epoch,
            version: self.version(),
            root_hash: txn_info.transaction_info().ensure_state_checkpoint_hash()?,
            chunks,
            proof: proof_handle,
        };

        let (manifest_handle, mut manifest_file) = self
            .storage
            .create_for_write(backup_handle, Self::manifest_name())
            .await?;
        manifest_file
            .write_all(&serde_json::to_vec(&manifest)?)
            .await?;
        manifest_file.shutdown().await?;

        let metadata = Metadata::new_state_snapshot_backup(
            self.epoch,
            self.version(),
            manifest_handle.clone(),
        );
        self.storage
            .save_metadata_line(&metadata.name(), &metadata.to_text_line()?)
            .await?;

        Ok(manifest_handle)
    }
}
