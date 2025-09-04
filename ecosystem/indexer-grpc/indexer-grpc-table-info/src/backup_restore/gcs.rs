// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    fs_ops::{create_tar_gz, unpack_tar_gz},
    generate_blob_name, BackupRestoreMetadata, JSON_FILE_TYPE, METADATA_FILE_NAME, TAR_FILE_TYPE,
};
use anyhow::Context;
use velor_logger::{error, info};
use futures::TryFutureExt;
use google_cloud_storage::{
    client::{Client, ClientConfig},
    http::{
        buckets::get::GetBucketRequest,
        objects::{
            download::Range,
            get::GetObjectRequest,
            upload::{Media, UploadObjectRequest, UploadType},
        },
        Error,
    },
};
use hyper::StatusCode;
use std::{borrow::Cow::Borrowed, env, path::PathBuf, time::Duration};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
    task,
};
use tokio_stream::StreamExt;

pub struct GcsBackupRestoreOperator {
    bucket_name: String,
    gcs_client: Client,
}

impl GcsBackupRestoreOperator {
    pub async fn new(bucket_name: String) -> Self {
        let gcs_config = ClientConfig::default()
            .with_auth()
            .await
            .expect("Failed to create GCS client.");
        let gcs_client = Client::new(gcs_config);
        Self {
            bucket_name,
            gcs_client,
        }
    }
}

impl GcsBackupRestoreOperator {
    pub async fn verify_storage_bucket_existence(&self) {
        info!(
            bucket_name = self.bucket_name,
            "Before gcs backup restore operator starts, verify the bucket exists."
        );

        self.gcs_client
            .get_bucket(&GetBucketRequest {
                bucket: self.bucket_name.to_string(),
                ..Default::default()
            })
            .await
            .unwrap_or_else(|_| panic!("Failed to get the bucket with name: {}", self.bucket_name));
    }

    pub async fn get_metadata(&self) -> Option<BackupRestoreMetadata> {
        match self.download_metadata_object().await {
            Ok(metadata) => Some(metadata),
            Err(Error::HttpClient(err)) => {
                if err.status() == Some(StatusCode::NOT_FOUND) {
                    None
                } else {
                    panic!("Error happens when accessing metadata file. {}", err);
                }
            },
            Err(e) => {
                panic!("Error happens when accessing metadata file. {}", e);
            },
        }
    }

    pub async fn create_default_metadata_if_absent(
        &self,
        expected_chain_id: u64,
    ) -> anyhow::Result<BackupRestoreMetadata> {
        match self.download_metadata_object().await {
            Ok(metadata) => {
                assert!(metadata.chain_id == expected_chain_id, "Chain ID mismatch.");
                Ok(metadata)
            },
            Err(Error::HttpClient(err)) => {
                let is_file_missing = err.status() == Some(StatusCode::NOT_FOUND);
                if is_file_missing {
                    self.update_metadata(expected_chain_id, 0)
                        .await
                        .expect("Update metadata failed.");
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

    async fn download_metadata_object(&self) -> Result<BackupRestoreMetadata, Error> {
        self.gcs_client
            .download_object(
                &GetObjectRequest {
                    bucket: self.bucket_name.clone(),
                    object: METADATA_FILE_NAME.to_string(),
                    ..Default::default()
                },
                &Range::default(),
            )
            .await
            .map(BackupRestoreMetadata::from)
    }

    pub async fn update_metadata(&self, chain_id: u64, epoch: u64) -> anyhow::Result<()> {
        let metadata = BackupRestoreMetadata::new(chain_id, epoch);
        loop {
            match self
                .gcs_client
                .upload_object(
                    &UploadObjectRequest {
                        bucket: self.bucket_name.clone(),
                        ..Default::default()
                    },
                    serde_json::to_vec(&metadata).unwrap(),
                    &UploadType::Simple(Media {
                        name: Borrowed(METADATA_FILE_NAME),
                        content_type: Borrowed(JSON_FILE_TYPE),
                        content_length: None,
                    }),
                )
                .await
            {
                Ok(_) => {
                    velor_logger::info!(
                        "[Table Info] Successfully updated metadata to GCS bucket: {}",
                        METADATA_FILE_NAME
                    );
                    return Ok(());
                },
                // https://cloud.google.com/storage/quotas
                // add retry logic due to: "Maximum rate of writes to the same object name: One write per second"
                Err(Error::Response(err)) if (err.is_retriable() && err.code == 429) => {
                    info!("Retried with rateLimitExceeded on gcs single object at epoch {} when updating the metadata", epoch);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                },
                Err(err) => {
                    anyhow::bail!("Failed to update metadata: {}", err);
                },
            }
        }
    }

    /// Backup the db snapshot to GCS bucket; this is a stateless operation, and
    /// all validation should be done before calling this function.
    pub async fn backup_db_snapshot_and_update_metadata(
        &self,
        chain_id: u64,
        epoch: u64,
        snapshot_path: PathBuf,
    ) -> anyhow::Result<()> {
        // chain id + epoch is the unique identifier for the snapshot.
        let snapshot_tar_file_name = format!("chain_id_{}_epoch_{}", chain_id, epoch);
        let snapshot_path_closure = snapshot_path.clone();
        velor_logger::info!(
            snapshot_tar_file_name = snapshot_tar_file_name.as_str(),
            "[Table Info] Starting to compress the folder.",
        );
        // If target path does not exist, wait and log.
        if !snapshot_path.exists() {
            velor_logger::warn!(
                snapshot_path = snapshot_path.to_str(),
                snapshot_tar_file_name = snapshot_tar_file_name.as_str(),
                epoch = epoch,
                "[Table Info] Directory does not exist. Waiting for the directory to be created."
            );
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            return Ok(());
        }
        let tar_file = task::spawn_blocking(move || {
            velor_logger::info!(
                snapshot_tar_file_name = snapshot_tar_file_name.as_str(),
                "[Table Info] Compressing the folder."
            );
            let result = create_tar_gz(snapshot_path_closure.clone(), &snapshot_tar_file_name);
            velor_logger::info!(
                snapshot_tar_file_name = snapshot_tar_file_name.as_str(),
                result = result.is_ok(),
                "[Table Info] Compressed the folder."
            );
            result
        })
        .await
        .context("Failed to spawn task to create snapshot backup file.")?
        .context("Failed to create tar.gz file in blocking task")?;
        velor_logger::info!(
            "[Table Info] Created snapshot tar file: {:?}",
            tar_file.file_name().unwrap()
        );

        // Open the file in async mode to stream it
        let file = File::open(&tar_file)
            .await
            .context("Failed to open gzipped tar file for reading")?;
        let file_stream = tokio_util::io::ReaderStream::new(file);

        let filename = generate_blob_name(chain_id, epoch);

        velor_logger::info!(
            "[Table Info] Uploading snapshot to GCS bucket: {}",
            filename
        );
        match self
            .gcs_client
            .upload_streamed_object(
                &UploadObjectRequest {
                    bucket: self.bucket_name.clone(),
                    ..Default::default()
                },
                file_stream,
                &UploadType::Simple(Media {
                    name: filename.clone().into(),
                    content_type: Borrowed(TAR_FILE_TYPE),
                    content_length: None,
                }),
            )
            .await
        {
            Ok(_) => {
                self.update_metadata(chain_id, epoch).await?;
                let snapshot_path_clone = snapshot_path.clone();
                fs::remove_file(&tar_file)
                    .and_then(|_| fs::remove_dir_all(snapshot_path_clone))
                    .await
                    .expect("Failed to clean up after db snapshot upload");
                velor_logger::info!(
                    "[Table Info] Successfully uploaded snapshot to GCS bucket: {}",
                    filename
                );
            },
            Err(err) => {
                error!("Failed to upload snapshot: {}", err);
                // TODO: better error handling, i.e., permanent failure vs transient failure.
                // For example, permission issue vs rate limit issue.
                anyhow::bail!("Failed to upload snapshot: {}", err);
            },
        };

        Ok(())
    }

    /// When fullnode is getting started, it will first restore its table info db by restoring most recent snapshot from gcs buckets.
    /// Download the right snapshot based on epoch to a local file and then unzip it and write to the indexer async v2 db.
    pub async fn restore_db_snapshot(
        &self,
        chain_id: u64,
        metadata: BackupRestoreMetadata,
        db_path: PathBuf,
        base_path: PathBuf,
    ) -> anyhow::Result<()> {
        assert!(metadata.chain_id == chain_id, "Chain ID mismatch.");

        let epoch = metadata.epoch;
        let epoch_based_filename = generate_blob_name(chain_id, epoch);

        match self
            .gcs_client
            .download_streamed_object(
                &GetObjectRequest {
                    bucket: self.bucket_name.clone(),
                    object: epoch_based_filename.clone(),
                    ..Default::default()
                },
                &Range::default(),
            )
            .await
        {
            Ok(mut stream) => {
                // Create a temporary file and write the stream to it directly
                let temp_file_name = "snapshot.tar.gz";
                let temp_file_path = base_path.join(temp_file_name);
                let temp_file_path_clone = temp_file_path.clone();
                let mut temp_file = File::create(&temp_file_path_clone).await?;
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(data) => temp_file.write_all(&data).await?,
                        Err(e) => return Err(anyhow::Error::new(e)),
                    }
                }
                temp_file.sync_all().await?;

                // Spawn blocking a thread to synchronously unpack gzipped tar file without blocking the async thread
                task::spawn_blocking(move || unpack_tar_gz(&temp_file_path_clone, &db_path))
                    .await?
                    .expect("Failed to unpack gzipped tar file");

                fs::remove_file(&temp_file_path)
                    .await
                    .context("Failed to remove temporary file after unpacking")?;
                Ok(())
            },
            Err(e) => Err(anyhow::Error::new(e)),
        }
    }
}
