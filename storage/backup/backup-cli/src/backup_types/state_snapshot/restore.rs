// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
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
        OTHER_TIMERS_SECONDS,
    },
    storage::{BackupStorage, FileHandle},
    utils::{
        read_record_bytes::ReadRecordBytes, storage_ext::BackupStorageExt, stream::StreamX,
        GlobalRestoreOptions, RestoreRunMode,
    },
};
use anyhow::{anyhow, ensure, Result};
use aptos_db::state_restore::StateSnapshotRestoreMode;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::StateSnapshotReceiver;
use aptos_types::{
    access_path::Path,
    ledger_info::LedgerInfoWithSignatures,
    on_chain_config::Features,
    proof::TransactionInfoWithProof,
    state_store::{
        state_key::{inner::StateKeyInner, StateKey},
        state_value::StateValue,
    },
    transaction::Version,
};
use aptos_vm_environment::prod_configs::{aptos_prod_verifier_config, LATEST_GAS_FEATURE_VERSION};
use clap::Parser;
use futures::{stream, TryStreamExt};
use move_binary_format::CompiledModule;
use move_bytecode_verifier::verify_module_with_config;
use std::sync::Arc;
use tokio::time::Instant;

#[derive(Parser)]
pub struct StateSnapshotRestoreOpt {
    #[clap(long = "state-manifest")]
    pub manifest_handle: FileHandle,
    #[clap(long = "state-into-version")]
    pub version: Version,
    #[clap(long)]
    pub validate_modules: bool,
    #[clap(long)]
    pub restore_mode: StateSnapshotRestoreMode,
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
    concurrent_downloads: usize,
    validate_modules: bool,
    restore_mode: StateSnapshotRestoreMode,
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
            concurrent_downloads: global_opt.concurrent_downloads,
            validate_modules: opt.validate_modules,
            restore_mode: opt.restore_mode,
        }
    }

    pub async fn run(self) -> Result<()> {
        let name = self.name();
        let start = Instant::now();
        info!("{} started. Manifest: {}", name, self.manifest_handle);
        self.run_impl()
            .await
            .map_err(|e| anyhow!("{} failed: {}", name, e))?;
        info!(time = start.elapsed().as_secs(), "{} succeeded.", name);
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

        let receiver = Arc::new(Mutex::new(Some(self.run_mode.get_state_restore_receiver(
            self.version,
            manifest.root_hash,
            self.restore_mode,
        )?)));

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

        let resume_point_opt = receiver.lock().as_mut().unwrap().previous_key_hash()?;
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

        let start_idx = chunks.first().map_or(0, |chunk| chunk.first_idx);

        let storage = self.storage.clone();
        let futs_iter = chunks.into_iter().enumerate().map(|(chunk_idx, chunk)| {
            let storage = storage.clone();
            async move {
                tokio::spawn(async move {
                    let blobs = Self::read_state_value(&storage, chunk.blobs.clone()).await?;
                    let proof = storage.load_bcs_file(&chunk.proof).await?;
                    Result::<_>::Ok((chunk_idx, chunk, blobs, proof))
                })
                .await?
            }
        });
        let con = self.concurrent_downloads;
        let mut futs_stream = stream::iter(futs_iter).buffered_x(con * 2, con);
        let mut start = None;
        while let Some((chunk_idx, chunk, mut blobs, proof)) = futs_stream.try_next().await? {
            start = start.or_else(|| Some(Instant::now()));
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["add_state_chunk"]);
            let receiver = receiver.clone();
            if self.validate_modules {
                blobs = tokio::task::spawn_blocking(move || {
                    Self::validate_modules(&blobs);
                    blobs
                })
                .await?;
            }
            tokio::task::spawn_blocking(move || {
                receiver.lock().as_mut().unwrap().add_chunk(blobs, proof)
            })
            .await??;
            leaf_idx.set(chunk.last_idx as i64);
            info!(
                chunk = chunk_idx,
                chunks_to_add = chunks_to_add,
                last_idx = chunk.last_idx,
                values_per_second = ((chunk.last_idx + 1 - start_idx) as f64
                    / start.as_ref().unwrap().elapsed().as_secs_f64())
                    as u64,
                "State chunk added.",
            );
        }

        tokio::task::spawn_blocking(move || receiver.lock().take().unwrap().finish()).await??;
        self.run_mode.finish();
        Ok(())
    }

    fn validate_modules(blob: &[(StateKey, StateValue)]) {
        // TODO: Instead of using default features, fetch them from the the state.
        let features = Features::default();

        let config = aptos_prod_verifier_config(LATEST_GAS_FEATURE_VERSION, &features);
        for (key, value) in blob {
            if let StateKeyInner::AccessPath(p) = key.inner() {
                if let Path::Code(module_id) = p.get_path() {
                    if let Ok(module) = CompiledModule::deserialize(value.bytes()) {
                        if let Err(err) = verify_module_with_config(&config, &module) {
                            error!("Module {:?} failed validation: {:?}", module_id, err);
                        }
                    } else {
                        error!("Module {:?} failed to deserialize", module_id);
                    }
                }
            }
        }
    }

    async fn read_state_value(
        storage: &Arc<dyn BackupStorage>,
        file_handle: FileHandle,
    ) -> Result<Vec<(StateKey, StateValue)>> {
        let mut file = storage.open_for_read(&file_handle).await?;

        let mut chunk = vec![];

        while let Some(record_bytes) = file.read_record_bytes().await? {
            chunk.push(bcs::from_bytes(&record_bytes)?);
        }

        Ok(chunk)
    }
}
