// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backup_types::{
        epoch_ending::restore::EpochHistory, state_snapshot::manifest::StateSnapshotBackup,
    },
    metrics::{
        restore::{
            STATE_SNAPSHOT_LEAF_INDEX, STATE_SNAPSHOT_TARGET_LEAF_INDEX, STATE_SNAPSHOT_VERSION,
        },
        verify::{
            VERIFY_STATE_SNAPSHOT_LEAF_INDEX, VERIFY_STATE_SNAPSHOT_TARGET_LEAF_INDEX,
            VERIFY_STATE_SNAPSHOT_VERSION,
        },
    },
    storage::{BackupStorage, FileHandle},
    utils::{
        read_record_bytes::ReadRecordBytes, storage_ext::BackupStorageExt, GlobalRestoreOptions,
        RestoreRunMode,
    },
};
use anyhow::{anyhow, ensure, Result};
use aptos_logger::prelude::*;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    proof::TransactionInfoWithProof,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use std::sync::Arc;
use storage_interface::StateSnapshotReceiver;
use structopt::StructOpt;
use tokio::time::Instant;

#[derive(StructOpt)]
pub struct StateSnapshotRestoreOpt {
    #[structopt(long = "state-manifest")]
    pub manifest_handle: FileHandle,
    #[structopt(long = "state-into-version")]
    pub version: Version,
}

pub struct StateSnapshotRestoreController {
    storage: Arc<dyn BackupStorage>,
    run_mode: Arc<RestoreRunMode>,
    /// State snapshot restores to this version.
    version: Version,
    manifest_handle: FileHandle,
    /// Global "target_version" for the entire restore process, if `version` is newer than this,
    /// nothing will be done, otherwise, this has no effect.
    target_version: Version,
    epoch_history: Option<Arc<EpochHistory>>,
}

impl StateSnapshotRestoreController {
    pub fn new(
        opt: StateSnapshotRestoreOpt,
        global_opt: GlobalRestoreOptions,
        storage: Arc<dyn BackupStorage>,
        epoch_history: Option<Arc<EpochHistory>>,
    ) -> Self {
        Self {
            storage,
            run_mode: global_opt.run_mode,
            version: opt.version,
            manifest_handle: opt.manifest_handle,
            target_version: global_opt.target_version,
            epoch_history,
        }
    }

    pub async fn run(self) -> Result<()> {
        let name = self.name();
        info!("{} started. Manifest: {}", name, self.manifest_handle);
        self.run_impl()
            .await
            .map_err(|e| anyhow!("{} failed: {}", name, e))?;
        info!("{} succeeded.", name);
        Ok(())
    }
}

impl StateSnapshotRestoreController {
    fn name(&self) -> String {
        format!("state snapshot {}", self.run_mode.name())
    }

    async fn run_impl(self) -> Result<()> {
        if self.version > self.target_version {
            warn!(
                "Trying to restore state snapshot to version {}, which is newer than the target version {}, skipping.",
                self.version,
                self.target_version,
            );
            return Ok(());
        }

        let manifest: StateSnapshotBackup =
            self.storage.load_json_file(&self.manifest_handle).await?;
        let (txn_info_with_proof, li): (TransactionInfoWithProof, LedgerInfoWithSignatures) =
            self.storage.load_bcs_file(&manifest.proof).await?;
        txn_info_with_proof.verify(li.ledger_info(), manifest.version)?;
        let state_root_hash = txn_info_with_proof
            .transaction_info()
            .ensure_state_checkpoint_hash()?;
        ensure!(
            state_root_hash == manifest.root_hash,
            "Root hash mismatch with that in proof. root hash: {}, expected: {}",
            manifest.root_hash,
            state_root_hash,
        );
        if let Some(epoch_history) = self.epoch_history.as_ref() {
            epoch_history.verify_ledger_info(&li)?;
        }

        let mut receiver = self
            .run_mode
            .get_state_restore_receiver(self.version, manifest.root_hash)?;

        let (ver_gauge, tgt_leaf_idx, leaf_idx) = if self.run_mode.is_verify() {
            (
                &VERIFY_STATE_SNAPSHOT_VERSION,
                &VERIFY_STATE_SNAPSHOT_TARGET_LEAF_INDEX,
                &VERIFY_STATE_SNAPSHOT_LEAF_INDEX,
            )
        } else {
            (
                &STATE_SNAPSHOT_VERSION,
                &STATE_SNAPSHOT_TARGET_LEAF_INDEX,
                &STATE_SNAPSHOT_LEAF_INDEX,
            )
        };

        ver_gauge.set(self.version as i64);
        tgt_leaf_idx.set(manifest.chunks.last().map_or(0, |c| c.last_idx as i64));
        let total_chunks = manifest.chunks.len();

        let resume_point_opt = receiver.previous_key_hash();
        let chunks = if let Some(resume_point) = resume_point_opt {
            manifest
                .chunks
                .into_iter()
                .skip_while(|chunk| chunk.last_key <= resume_point)
                .collect()
        } else {
            manifest.chunks
        };
        if chunks.len() < total_chunks {
            info!(
                chunks_to_add = chunks.len(),
                total_chunks = total_chunks,
                "Resumed state snapshot restore."
            )
        };
        let chunks_to_add = chunks.len();

        let start = Instant::now();
        let start_idx = chunks.first().map_or(0, |chunk| chunk.first_idx);
        for (i, chunk) in chunks.into_iter().enumerate() {
            let blobs = self.read_state_value(chunk.blobs).await?;
            let proof = self.storage.load_bcs_file(&chunk.proof).await?;
            info!("State chunk loaded.");
            receiver.add_chunk(blobs, proof)?;
            info!(
                chunk = i,
                chunks_to_add = chunks_to_add,
                last_idx = chunk.last_idx,
                values_per_second =
                    (chunk.last_idx + 1 - start_idx) as u64 / start.elapsed().as_secs(),
                "State chunk added.",
            );

            leaf_idx.set(chunk.last_idx as i64);
        }

        receiver.finish()?;
        self.run_mode.finish();
        Ok(())
    }

    async fn read_state_value(
        &self,
        file_handle: FileHandle,
    ) -> Result<Vec<(StateKey, StateValue)>> {
        let mut file = self.storage.open_for_read(&file_handle).await?;

        let mut chunk = vec![];

        while let Some(record_bytes) = file.read_record_bytes().await? {
            chunk.push(bcs::from_bytes(&record_bytes)?);
        }

        Ok(chunk)
    }
}
