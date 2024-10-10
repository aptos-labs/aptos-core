// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{ensure, Context, Result};
use aptos_indexer_grpc_utils::compression_util::{FileEntry, StorageFormat};
use aptos_metrics_core::{register_int_counter, IntCounter};
use cloud_storage::{Client, Error as CloudStorageError};
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub static FILE_DIFF_COUNTER: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_grpc_file_checker_file_diff",
        "Count of the files that are different.",
    )
    .unwrap()
});

const PROGRESS_FILE_NAME: &str = "file_checker_progress.json";
const METADATA_FILE_NAME: &str = "metadata.json";

/// Checker compares the data in the existing bucket with the data in the new bucket.
/// The progress is saved in a file under the new bucket.
pub struct Processor {
    /// Existing bucket name.
    pub existing_bucket_name: String,
    /// New bucket name; this job is to make sure the data in the new bucket is correct.
    pub new_bucket_name: String,
    /// The version to start from. This is for **bootstrapping** the file checker only.
    pub starting_version: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressFile {
    file_checker_version: u64,
    file_checker_chain_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataFile {
    chain_id: u64,
}

impl Processor {
    pub async fn run(&self) -> Result<()> {
        let (client, mut progress_file) = self.init().await?;

        loop {
            let current_version = progress_file.file_checker_version;

            let file_name =
                FileEntry::build_key(current_version, StorageFormat::Lz4CompressedProto);
            let existing_file =
                download_raw_file(&client, &self.existing_bucket_name, &file_name).await?;
            let new_file = download_raw_file(&client, &self.new_bucket_name, &file_name).await?;
            if existing_file.is_none() || new_file.is_none() {
                // Wait for the next file to be uploaded.
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                continue;
            }
            // Compare the files.
            let existing_file = existing_file.unwrap();
            let new_file = new_file.unwrap();
            if existing_file != new_file {
                // Files are different.
                tracing::error!("Files are different: {}", file_name);
                FILE_DIFF_COUNTER.inc();

                // Sleep for a while to allow metrics to be updated.
                tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;
                panic!("Files are different: {}", file_name);
            }
            progress_file.file_checker_version += 1000;
            // Upload the progress file.
            let progress_file_bytes =
                serde_json::to_vec(&progress_file).context("Failed to serialize progress file.")?;
            client
                .object()
                .create(
                    &self.new_bucket_name,
                    progress_file_bytes,
                    PROGRESS_FILE_NAME,
                    "application/json",
                )
                .await
                .context("Update progress file failure")?;
        }
    }

    /// Initialize the processor.
    pub async fn init(&self) -> Result<(Client, ProgressFile)> {
        let client = Client::new();

        // All errors are considered fatal: files must exist for the processor to work.
        let existing_metadata =
            download_file::<MetadataFile>(&client, &self.existing_bucket_name, METADATA_FILE_NAME)
                .await
                .context("Failed to get metadata.")?
                .expect("Failed to download metadata file");
        let new_metadata =
            download_file::<MetadataFile>(&client, &self.new_bucket_name, METADATA_FILE_NAME)
                .await
                .context("Failed to get metadata.")?
                .expect("Failed to download metadata file");

        // Ensure the chain IDs match.
        ensure!(
            existing_metadata.chain_id == new_metadata.chain_id,
            "Chain IDs do not match: {} != {}",
            existing_metadata.chain_id,
            new_metadata.chain_id
        );

        let progress_file =
            download_file::<ProgressFile>(&client, &self.new_bucket_name, PROGRESS_FILE_NAME)
                .await
                .context("Failed to get progress file.")?
                .unwrap_or(ProgressFile {
                    file_checker_version: self.starting_version,
                    file_checker_chain_id: existing_metadata.chain_id,
                });
        // Ensure the chain IDs match.
        ensure!(
            existing_metadata.chain_id == progress_file.file_checker_chain_id,
            "Chain IDs do not match: {} != {}",
            existing_metadata.chain_id,
            progress_file.file_checker_chain_id
        );

        Ok((client, progress_file))
    }
}

async fn download_raw_file(
    client: &Client,
    bucket_name: &str,
    file_name: &str,
) -> Result<Option<Vec<u8>>> {
    let file = client.object().download(bucket_name, file_name).await;
    match file {
        Ok(file) => Ok(Some(file)),
        Err(CloudStorageError::Google(e)) => {
            if e.error.code == 404 {
                Ok(None)
            } else {
                Err(e.into())
            }
        },
        Err(err) => Err(err.into()),
    }
}

async fn download_file<T>(client: &Client, bucket_name: &str, file_name: &str) -> Result<Option<T>>
where
    T: DeserializeOwned,
{
    let file = download_raw_file(client, bucket_name, file_name).await?;
    match file {
        Some(file) => {
            let file = serde_json::from_slice(&file).context("Failed to parse file.")?;
            Ok(Some(file))
        },
        None => Ok(None),
    }
}
