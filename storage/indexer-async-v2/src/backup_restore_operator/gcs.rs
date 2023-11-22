// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{BackupRestoreMetadata, BackupRestoreOperator, METADATA_FILE_NAME, generate_blob_name};
use anyhow::{Context, Error};
use aptos_logger::{error, info};
use aptos_storage_interface::DbWriter;
use cloud_storage::{Bucket, Object};
use std::{
    env,
    path::PathBuf,
    sync::{atomic::{AtomicU64, Ordering}, Arc},
    io::{Write, BufWriter}, fs,
};
use std::fs::File;
use tar::{Builder, Archive};
use flate2::{write::GzEncoder, read::GzDecoder, Compression};

const JSON_FILE_TYPE: &str = "application/json";
const TAR_FILE_TYPE: &str = "application/gzip";
const TEMPORARY_MARKER: u64 = u64::MAX; // A value that represents an ongoing update

pub struct GcsBackupRestoreOperator {
    bucket_name: String,
    metadata_epoch: AtomicU64,
}

impl GcsBackupRestoreOperator {
    pub fn new(bucket_name: String) -> Self {
        Self {
            bucket_name,
            metadata_epoch: AtomicU64::new(0),
        }
    }
}

#[async_trait::async_trait]
impl BackupRestoreOperator for GcsBackupRestoreOperator {
    async fn verify_storage_bucket_existence(&self) {
        tracing::info!(
            bucket_name = self.bucket_name,
            "Before gcs operator starts, verify the bucket exists."
        );
        Bucket::read(&self.bucket_name)
            .await
            .expect("Failed to read bucket.");
    }

    async fn get_metadata(&self) -> Option<BackupRestoreMetadata> {
        match Object::download(&self.bucket_name, METADATA_FILE_NAME).await {
            Ok(metadata) => {
                let metadata: BackupRestoreMetadata =
                    serde_json::from_slice(&metadata).expect("Expected metadata to be valid JSON.");
                Some(metadata)
            },
            Err(cloud_storage::Error::Other(err)) => {
                if err.contains("No such object: ") {
                    None
                } else {
                    panic!(
                        "Error happens when accessing metadata file. {}",
                        err
                    );
                }
            },
            Err(e) => {
                panic!(
                    "Error happens when accessing metadata file. {}",
                    e
                );
            },
        }
    }

    async fn create_default_metadata_if_absent(
        &self,
        expected_chain_id: u64,
    ) -> anyhow::Result<BackupRestoreMetadata> {
        match Object::download(&self.bucket_name, METADATA_FILE_NAME).await {
            Ok(metadata) => {
                let metadata: BackupRestoreMetadata =
                    serde_json::from_slice(&metadata).expect("Expected metadata to be valid JSON.");
                anyhow::ensure!(metadata.chain_id == expected_chain_id, "Chain ID mismatch.");
                self.set_metadata_epoch(metadata.epoch);
                Ok(metadata)
            },
            Err(cloud_storage::Error::Other(err)) => {
                let is_file_missing = err.contains("No such object: ");
                if is_file_missing {
                    self.update_metadata(expected_chain_id, 0)
                        .await
                        .expect("Update metadata failed.");
                    self.set_metadata_epoch(0);
                    Ok(BackupRestoreMetadata::new(expected_chain_id, 0))
                } else {
                    Err(anyhow::Error::msg(format!(
                        "Metadata not found or gcs operator is not in write mode. {}",
                        err
                    )))
                }
            },
            Err(err) => Err(anyhow::Error::from(err)),
        }
    }

    async fn update_metadata(&self, chain_id: u64, epoch: u64) -> anyhow::Result<()> {
        let metadata = BackupRestoreMetadata::new(chain_id, epoch);

        match Object::create(
            self.bucket_name.as_str(),
            serde_json::to_vec(&metadata).unwrap(),
            METADATA_FILE_NAME,
            JSON_FILE_TYPE,
        )
        .await
        {
            Ok(_) => {
                return Ok(());
            },
            Err(err) => {
                anyhow::bail!("Failed to update metadata: {}", err);
            },
        }
    }

    async fn try_upload_snapshot(&self, chain_id: u64, block_event_epoch: u64,db_writer: Arc<dyn DbWriter>, snapshot_path: PathBuf) -> anyhow::Result<()> {
        // Attempt to atomically set the metadata_epoch to TEMPORARY_MARKER
        let current_epoch = self.get_metadata_epoch();
        if current_epoch < block_event_epoch {
            match self.metadata_epoch.compare_exchange(
                current_epoch, 
                TEMPORARY_MARKER, // Set to TEMPORARY_MARKER during update
                Ordering::Relaxed, 
                Ordering::Relaxed
            ) {
                Ok(_) => {
                    // Proceed with the upload
                    let _ = db_writer.clone().create_checkpoint(snapshot_path.clone());

                    self.upload_snapshot(chain_id, block_event_epoch, snapshot_path).await?;

                    // After upload, set to the actual new epoch
                    self.set_metadata_epoch(block_event_epoch);
                },
                Err(_) => {
                    info!("Another thread is already handling the snapshot upload or it's completed");
                }
            }
        }
        Ok(())
    }

    async fn upload_snapshot(
        &self,
        chain_id: u64,
        epoch: u64,
        snapshot_path: PathBuf,
    ) -> anyhow::Result<()> {
        let bucket_name = self.bucket_name.clone();
        let (tar_file, _tar_file_name) = create_tar_gz(snapshot_path.clone(), &epoch.to_string())?;
        let buffer = std::fs::read(tar_file.as_path()).context("Failed to read gzipped tar file").unwrap();
        let filename = generate_blob_name(epoch);
        match Object::create(bucket_name.as_str(), buffer.clone(), &filename, TAR_FILE_TYPE).await {
            Ok(_) => {
                if tar_file.exists() {
                    if let Err(e) = fs::remove_file(&tar_file) {
                        if e.kind() != std::io::ErrorKind::NotFound {
                            return Err(e).context("Failed to remove local gzipped tar file");
                        }
                    }
                    if let Err(e) = fs::remove_dir(snapshot_path.clone()) {
                        if e.kind() != std::io::ErrorKind::NotFound {
                            return Err(e).context(format!("Failed to remove local snapshot folder at the {} epoch", epoch.to_string()));
                        }
                    }
                }
                self.update_metadata(chain_id, epoch).await?;
                return Ok(());
            },
            Err(err) => {
                anyhow::bail!("Failed to upload snapshot: {}", err);
            },
        }
    }

    async fn restore_snapshot(&self, chain_id: u64, db_path: PathBuf) -> anyhow::Result<()> {
        let metadata = self.get_metadata().await;
        if metadata.is_none() {
            info!("Trying to restore from gcs backup but metadata.json file does not exist");
            return Ok(());
        }
        let metadata = metadata.unwrap();
        anyhow::ensure!(metadata.chain_id == chain_id, "Chain ID mismatch.");
        let epoch = metadata.epoch;
        self.set_metadata_epoch(epoch);
        if epoch == 0 {
            info!("Trying to restore from gcs bap but latest epoch is 0");
            return Ok(());
        }
        let epoch_file_name = generate_blob_name(epoch);

        match Object::download(&self.bucket_name, &epoch_file_name).await {
            Ok(snapshot) => {
                let temp_file_name = "snapshot.tar.gz";
                let temp_file_path = db_path.join(temp_file_name);
                write_snapshot_to_file(&snapshot, &temp_file_path)?;
                unpack_tar_gz(&temp_file_path, &db_path)?;
                if temp_file_path.exists() {
                    if let Err(e) = fs::remove_file(&temp_file_path) {
                        if e.kind() != std::io::ErrorKind::NotFound {
                            return Err(e).context("Failed to remove temp gzipped tar file");
                        }
                    }
                }
                Ok(())
            },
            Err(e) => Err(anyhow::Error::new(e)),
        }
    }

    fn get_metadata_epoch(&self) -> u64 {
        self.metadata_epoch.load(Ordering::Relaxed)
    }

    fn set_metadata_epoch(&self, epoch: u64) {
        self.metadata_epoch.store(epoch, Ordering::Relaxed)
    }
}

fn create_tar_gz(dir_path: PathBuf, backup_file_name: &str) -> Result<(PathBuf, String), Error> {
    let tar_file_name = format!("{}.tar.gz", backup_file_name);
    let mut tar_file_path = dir_path.clone();
    tar_file_path.set_file_name(&tar_file_name);
    
    let tar_file = File::create(&tar_file_path)?;
    let gz_encoder = GzEncoder::new(tar_file, Compression::default());
    let tar_data = BufWriter::new(gz_encoder);
    let mut tar_builder = Builder::new(tar_data);
    tar_builder.append_dir_all(".", &dir_path)?;
    drop(tar_builder.into_inner()?);
    Ok((tar_file_path, tar_file_name))
}

fn write_snapshot_to_file(snapshot: &[u8], file_path: &PathBuf) -> anyhow::Result<()> {
    let mut temp_file = File::create(file_path)?;
    temp_file.write_all(snapshot)?;
    temp_file.flush()?; // Ensure all data is written
    Ok(())
}

fn unpack_tar_gz(file_path: &PathBuf, db_path: &PathBuf) -> anyhow::Result<()> {
    let file = File::open(file_path)?;
    let gz_decoder = GzDecoder::new(file);
    let mut archive = Archive::new(gz_decoder);

    match archive.unpack(db_path) {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to unpack gzipped archive: {:?}", e);
            Err(anyhow::Error::new(e))
        },
    }
}
