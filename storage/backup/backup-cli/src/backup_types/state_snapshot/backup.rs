// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup_types::state_snapshot::manifest::{StateSnapshotBackup, StateSnapshotChunk},
    metadata::Metadata,
    storage::{BackupHandleRef, BackupStorage, FileHandle, ShellSafeName},
    utils::{
        backup_service_client::BackupServiceClient, read_record_bytes::ReadRecordBytes,
        should_cut_chunk, storage_ext::BackupStorageExt, GlobalBackupOpt,
    },
};
use anyhow::{anyhow, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_logger::prelude::*;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    proof::TransactionInfoWithProof,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use bytes::Bytes;
use clap::Parser;
use once_cell::sync::Lazy;
use std::{convert::TryInto, str::FromStr, sync::Arc};
use tokio::io::AsyncWriteExt;
use tokio::time::Instant;

#[derive(Parser)]
pub struct StateSnapshotBackupOpt {
    #[clap(
        long = "state-snapshot-epoch",
        help = "Epoch at the end of which a state snapshot is to be taken."
    )]
    pub epoch: u64,
}

pub struct StateSnapshotBackupController {
    epoch: u64,
    version: Option<Version>, // initialize before using
    max_chunk_size: usize,
    client: Arc<BackupServiceClient>,
    storage: Arc<dyn BackupStorage>,
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

        let mut chunks = vec![];

        let mut state_snapshot_file = self.client.get_state_snapshot(self.version()).await?;
        let mut prev_record_bytes = state_snapshot_file
            .read_record_bytes()
            .await?
            .ok_or_else(|| anyhow!("State is empty."))?;
        let mut chunk_bytes = (prev_record_bytes.len() as u32).to_be_bytes().to_vec();
        chunk_bytes.extend(&prev_record_bytes);
        let mut chunk_first_key = Self::parse_key(&prev_record_bytes)?;
        let mut current_idx: usize = 0;
        let mut chunk_first_idx: usize = 0;

        let start = Instant::now();
        while let Some(record_bytes) = state_snapshot_file.read_record_bytes().await? {
            if should_cut_chunk(&chunk_bytes, &record_bytes, self.max_chunk_size) {
                let chunk = self
                    .write_chunk(
                        &backup_handle,
                        &chunk_bytes,
                        chunk_first_idx,
                        current_idx,
                        chunk_first_key,
                        Self::parse_key(&prev_record_bytes)?,
                    )
                    .await?;
                chunks.push(chunk);
                chunk_bytes = vec![];
                chunk_first_idx = current_idx + 1;
                chunk_first_key = Self::parse_key(&record_bytes)?;

                info!(
                    last_idx = current_idx,
                    values_per_second =
                        ((current_idx + 1) as f64 / start.elapsed().as_secs_f64()) as u64,
                    "Chunk written."
                );
            }

            current_idx += 1;
            chunk_bytes.extend(&(record_bytes.len() as u32).to_be_bytes());
            chunk_bytes.extend(&record_bytes);
            prev_record_bytes = record_bytes;
        }

        assert!(!chunk_bytes.is_empty());
        let chunk = self
            .write_chunk(
                &backup_handle,
                &chunk_bytes,
                chunk_first_idx,
                current_idx,
                chunk_first_key,
                Self::parse_key(&prev_record_bytes)?,
            )
            .await?;
        chunks.push(chunk);

        self.write_manifest(&backup_handle, chunks).await
    }
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

    fn parse_key(record: &Bytes) -> Result<HashValue> {
        let (key, _): (StateKey, StateValue) = bcs::from_bytes(record)?;
        Ok(key.hash())
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
        chunk_bytes: &[u8],
        first_idx: usize,
        last_idx: usize,
        first_key: HashValue,
        last_key: HashValue,
    ) -> Result<StateSnapshotChunk> {
        let (chunk_handle, mut chunk_file) = self
            .storage
            .create_for_write(backup_handle, &Self::chunk_name(first_idx))
            .await?;
        chunk_file.write_all(chunk_bytes).await?;
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
