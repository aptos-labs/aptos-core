// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    generate_blob_name, BackupRestoreMetadata, JSON_FILE_TYPE, METADATA_FILE_NAME, TAR_FILE_TYPE,
};
use anyhow::Context;
use aptos_logger::{error, info};
use aptos_storage_interface::DbWriter;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
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
use std::{
    borrow::Cow::Borrowed,
    env, fs,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tar::{Archive, Builder};

pub struct GcsBackupRestoreOperator {
    bucket_name: String,
    metadata_epoch: AtomicU64,
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
            metadata_epoch: AtomicU64::new(0),
            gcs_client,
        }
    }
}

impl GcsBackupRestoreOperator {
    pub async fn verify_storage_bucket_existence(&self) {
        info!(
            bucket_name = self.bucket_name,
            "Before gcs operator starts, verify the bucket exists."
        );

        self.gcs_client
            .get_bucket(&GetBucketRequest {
                bucket: self.bucket_name.to_string(),
                ..Default::default()
            })
            .await
            .expect("Failed to get the bucket");
    }

    pub async fn get_metadata(&self) -> Option<BackupRestoreMetadata> {
        match self
            .gcs_client
            .download_object(
                &GetObjectRequest {
                    bucket: self.bucket_name.clone(),
                    object: METADATA_FILE_NAME.to_string(),
                    ..Default::default()
                },
                &Range::default(),
            )
            .await
        {
            Ok(metadata) => {
                let metadata: BackupRestoreMetadata =
                    serde_json::from_slice(&metadata).expect("Expected metadata to be valid JSON.");
                Some(metadata)
            },
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
        match self
            .gcs_client
            .download_object(
                &GetObjectRequest {
                    bucket: self.bucket_name.clone(),
                    object: METADATA_FILE_NAME.to_string(),
                    ..Default::default()
                },
                &Range::default(),
            )
            .await
        {
            Ok(metadata) => {
                let metadata: BackupRestoreMetadata =
                    serde_json::from_slice(&metadata).expect("Expected metadata to be valid JSON.");
                anyhow::ensure!(metadata.chain_id == expected_chain_id, "Chain ID mismatch.");
                self.set_metadata_epoch(metadata.epoch);
                Ok(metadata)
            },
            Err(Error::HttpClient(err)) => {
                let is_file_missing = err.status() == Some(StatusCode::NOT_FOUND);
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

    pub async fn upload_snapshot(
        &self,
        chain_id: u64,
        epoch: u64,
        db_writer: Arc<dyn DbWriter>,
        snapshot_path: PathBuf,
    ) -> anyhow::Result<()> {
        // reading epoch from gcs metadata is too slow, so updating the local var
        self.set_metadata_epoch(epoch);

        // rocksdb will create a checkpoint to take a snapshot of full db and then save it to snapshot_path
        db_writer
            .clone()
            .create_checkpoint(&snapshot_path)
            .expect(&format!("DB checkpoint failed at epoch {}", epoch));

        // create a gzipped tar file by compressing a folder into a single file
        let (tar_file, _tar_file_name) = create_tar_gz(snapshot_path.clone(), &epoch.to_string())?;
        let buffer = std::fs::read(tar_file.as_path())
            .context("Failed to read gzipped tar file")
            .unwrap();
        let filename = generate_blob_name(epoch);
        // once object is successfully created in the gcs bucket, remove these temp file and folder
        match self
            .gcs_client
            .upload_object(
                &UploadObjectRequest {
                    bucket: self.bucket_name.clone(),
                    ..Default::default()
                },
                buffer.clone(),
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
                fs::remove_file(&tar_file).unwrap_or(());
                fs::remove_dir_all(&snapshot_path).unwrap_or(());
            },
            Err(err) => {
                error!("Failed to upload snapshot: {}", err);
            },
        };

        Ok(())
    }

    /// When fullnode is getting started, it will first restore its table info db by restoring most recent snapshot from gcs buckets.
    /// If there's no metadata json file in gcs, we will create a default one with epoch 0 and return early since there's no snapshot to restore from.
    /// If there is metadata json file in gcs, download the most recent snapshot to a local file and then unzip it and write to the indexer async v2 db.
    pub async fn restore_snapshot(
        &self,
        chain_id: u64,
        db_path: PathBuf,
        base_path: PathBuf,
    ) -> anyhow::Result<()> {
        let metadata = self.get_metadata().await;
        if metadata.is_none() {
            info!("Trying to restore from gcs backup but metadata.json file does not exist, creating metadata now...");
            self.create_default_metadata_if_absent(chain_id)
                .await
                .expect("Creating default metadata failed");
            return Ok(());
        }

        let metadata = metadata.unwrap();
        anyhow::ensure!(metadata.chain_id == chain_id, "Chain ID mismatch.");

        let epoch = metadata.epoch;
        self.set_metadata_epoch(epoch);
        if epoch == 0 {
            info!("Trying to restore from gcs bap but latest backup epoch is 0");
            return Ok(());
        }

        let epoch_based_filename = generate_blob_name(epoch);

        match self
            .gcs_client
            .download_object(
                &GetObjectRequest {
                    bucket: self.bucket_name.clone(),
                    object: epoch_based_filename.clone(),
                    ..Default::default()
                },
                &Range::default(),
            )
            .await
        {
            Ok(snapshot) => {
                let temp_file_name = "snapshot.tar.gz";
                let temp_file_path = base_path.join(temp_file_name);
                write_snapshot_to_file(&snapshot, &temp_file_path)?;
                unpack_tar_gz(&temp_file_path, &db_path)?;
                fs::remove_file(&temp_file_path).unwrap_or(());
                Ok(())
            },
            Err(e) => Err(anyhow::Error::new(e)),
        }
    }

    pub fn set_metadata_epoch(&self, epoch: u64) {
        self.metadata_epoch.store(epoch, Ordering::Relaxed)
    }

    pub fn get_metadata_epoch(&self) -> u64 {
        self.metadata_epoch.load(Ordering::Relaxed)
    }
}

/// Creates a tar.gz archive from the db snapshot directory
fn create_tar_gz(
    dir_path: PathBuf,
    backup_file_name: &str,
) -> Result<(PathBuf, String), anyhow::Error> {
    let tar_file_name = format!("{}.tar.gz", backup_file_name);
    let mut tar_file_path = dir_path.clone();
    tar_file_path.set_file_name(&tar_file_name);

    let tar_file = File::create(&tar_file_path)?;
    let gz_encoder = GzEncoder::new(tar_file, Compression::best());
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

/// Unpack a tar.gz archive to a specified directory
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use tempfile::tempdir;

    #[test]
    fn test_create_unpack_tar_gz_and_preserves_content() -> anyhow::Result<()> {
        // Create a temporary directory and a file within it
        let dir_to_compress = tempdir()?;
        let file_path = dir_to_compress.path().join("testfile.txt");
        let test_content = "Sample content";
        let mut file = File::create(&file_path)?;
        writeln!(file, "{}", test_content)?;

        // Create a tar.gz file from the directory
        let (tar_gz_path, _) = create_tar_gz(dir_to_compress.path().to_path_buf(), "testbackup")?;
        assert!(tar_gz_path.exists());

        // Create a new temporary directory to unpack the tar.gz file
        let unpack_dir = tempdir()?;
        unpack_tar_gz(&tar_gz_path, &unpack_dir.path().to_path_buf())?;

        // Verify the file is correctly unpacked
        let unpacked_file_path = unpack_dir.path().join("testfile.txt");
        assert!(unpacked_file_path.exists());

        // Read content from the unpacked file
        let mut unpacked_file = File::open(unpacked_file_path)?;
        let mut unpacked_content = String::new();
        unpacked_file.read_to_string(&mut unpacked_content)?;

        // Assert that the original content is equal to the unpacked content
        assert_eq!(unpacked_content.trim_end(), test_content);

        Ok(())
    }
}
