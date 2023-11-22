// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{storage::copy_directory, BackupRestoreOperator, FILE_FOLDER_NAME};
use crate::backup_restore_operator::{BackupRestoreMetadata, METADATA_FILE_NAME};
use anyhow::Context;
use aptos_storage_interface::DbWriter;
use std::{
    path::PathBuf,
    sync::{atomic::{AtomicU64, Ordering}, Arc},
};
use tokio::fs::{self};
use tracing::info;

const TEMPORARY_MARKER: u64 = u64::MAX; // A value that represents an ongoing update

pub struct LocalBackupRestoreOperator {
    path: PathBuf,
    metadata_epoch: AtomicU64,
}

impl LocalBackupRestoreOperator {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            metadata_epoch: AtomicU64::new(0),
        }
    }
}

#[async_trait::async_trait]
impl BackupRestoreOperator for LocalBackupRestoreOperator {
    async fn verify_storage_bucket_existence(&self) {
        tracing::info!(
            bucket_name = self.path.to_str().unwrap(),
            "Before local operator starts, verify the local file path exists."
        );
        if !self.path.exists() {
            panic!("Local file path does not exist.");
        }
    }

    async fn get_metadata(&self) -> Option<BackupRestoreMetadata> {
        let metadata_path = self.path.join(METADATA_FILE_NAME);
        match tokio::fs::read(metadata_path).await {
            Ok(metadata) => {
                let metadata: BackupRestoreMetadata =
                    serde_json::from_slice(&metadata).expect("Expected metadata to be valid JSON.");
                Some(metadata)
            },
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    None
                } else {
                    panic!(
                        "Error happens when accessing metadata file. {}",
                        err
                    );
                }
            },
        }
    }

    async fn create_default_metadata_if_absent(
        &self,
        expected_chain_id: u64,
    ) -> anyhow::Result<BackupRestoreMetadata> {
        let metadata_path = self.path.join(METADATA_FILE_NAME);
        match tokio::fs::read(metadata_path).await {
            Ok(metadata) => {
                let metadata: BackupRestoreMetadata =
                    serde_json::from_slice(&metadata).expect("Expected metadata to be valid JSON.");
                anyhow::ensure!(metadata.chain_id == expected_chain_id, "Chain ID mismatch.");
                self.metadata_epoch.store(metadata.epoch, Ordering::Relaxed);
                Ok(metadata)
            },
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    info!("Local file is empty. Creating metadata file.");
                    self.update_metadata(expected_chain_id, 0)
                        .await
                        .expect("Update metadata failed.");
                    self.metadata_epoch.store(0, Ordering::Relaxed);
                    Ok(BackupRestoreMetadata::new(expected_chain_id, 0))
                } else {
                    Err(anyhow::Error::msg(format!(
                        "Metadata not found or local file operator is not in write mode. {}",
                        err
                    )))
                }
            },
        }
    }

    async fn update_metadata(&self, chain_id: u64, epoch: u64) -> anyhow::Result<()> {
        let metadata = BackupRestoreMetadata::new(chain_id, epoch);
        let metadata_path = self.path.join(METADATA_FILE_NAME);
        info!(
            "Updating metadata file {} @ epoch {}",
            metadata_path.display(),
            epoch
        );
        match tokio::fs::write(metadata_path, serde_json::to_vec(&metadata).unwrap()).await {
            Ok(_) => {
                self.metadata_epoch.store(epoch, Ordering::Relaxed);
                Ok(())
            },
            Err(err) => Err(anyhow::Error::from(err)),
        }
    }

    async fn upload_snapshot(
        &self,
        chain_id: u64,
        epoch: u64,
        snapshot_path: PathBuf,
    ) -> anyhow::Result<()> {
        let files_dir = self.path.join(FILE_FOLDER_NAME);
        let write_path = files_dir.join(epoch.to_string());
        if !write_path.exists() {
            tokio::fs::create_dir_all(&write_path).await?;
        }

        let _ = copy_directory(
            snapshot_path
                .to_str()
                .expect("Snapshot path should exist when uploading to snapshot folder"),
            write_path
                .to_str()
                .expect("Write path should exist when uploading to snapshot folder"),
        );

        self.update_metadata(chain_id, epoch).await?;

        Ok(())
    }

    async fn restore_snapshot(&self, chain_id: u64, db_path: PathBuf) -> anyhow::Result<()> {
        let metadata = self.get_metadata().await;
        if metadata.is_none() {
            info!("Trying to restore but metadata does not exist");
            return Ok(());
        }
        anyhow::ensure!(metadata.unwrap().chain_id == chain_id, "Chain ID mismatch.");
        let epoch = metadata.unwrap().epoch;
        self.metadata_epoch.store(epoch, Ordering::Relaxed);
        let epoch_path = self.path.join(epoch.to_string());

        if !epoch_path.exists() {
            info!("Snapshot directory does not exist for epoch: {}", epoch);
            return Ok(());
        }

        if db_path.exists() {
            fs::remove_dir_all(&db_path)
                .await
                .context("Failed to remove existing db_path directory")?;
        }

        let _ = copy_directory(epoch_path.to_str().expect(""), db_path.to_str().expect(""));

        Ok(())
    }

    fn get_metadata_epoch(&self) -> u64 {
        self.metadata_epoch.load(Ordering::Relaxed)
    }

    fn set_metadata_epoch(&self, epoch: u64) {
        self.metadata_epoch.store(epoch, Ordering::Relaxed)
    }

    async fn try_upload_snapshot(&self, chain_id: u64, block_event_epoch: u64, _db_writer: Arc<dyn DbWriter>, snapshot_path: PathBuf) -> anyhow::Result<()> {
        // Attempt to atomically set the metadata_epoch to TEMPORARY_MARKER
        let current_epoch = self.metadata_epoch.load(Ordering::Relaxed);
        if current_epoch < block_event_epoch {
            match self.metadata_epoch.compare_exchange(
                current_epoch, 
                TEMPORARY_MARKER, // Set to TEMPORARY_MARKER during update
                Ordering::Relaxed, 
                Ordering::Relaxed
            ) {
                Ok(_) => {
                    // Proceed with the upload
                    self.upload_snapshot(chain_id, block_event_epoch, snapshot_path).await?;

                    // After upload, set to the actual new epoch
                    self.metadata_epoch.store(block_event_epoch, Ordering::Relaxed);
                },
                Err(_) => {
                    // Another thread is already handling the upload or it's completed
                }
            }
        }
        Ok(())
    }
}
