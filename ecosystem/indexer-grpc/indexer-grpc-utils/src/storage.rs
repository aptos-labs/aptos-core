// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use gcloud_storage::{
    client::{google_cloud_auth::credentials::CredentialsFile, Client, ClientConfig},
    http::objects::{
        download::Range,
        get::GetObjectRequest,
        upload::{Media, UploadObjectRequest, UploadType},
    },
};
use serde::{Deserialize, Serialize};

// The maximum number of transactions to store in a single blob.
pub const BLOB_TRANSACTION_CHUNK_SIZE: u64 = 1_000;
// The name of the folder in the bucket where blobs are stored.
pub(crate) const BLOB_FOLDER_NAME: &str = "blobs";
// Metadata file name for file store.
const METADATA_FILE_NAME: &str = "metadata.json";
const JSON_FILE_TYPE: &str = "application/json";

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct FileStoreMetadata {
    /// The chain_id for the file store.
    pub chain_id: u64,

    /// Current version of the file store.
    pub version: u64,
}

async fn create_gcs_client() -> Client {
    let config = match std::env::var("SERVICE_ACCOUNT") {
        Ok(service_account_path) => {
            let creds = CredentialsFile::new_from_file(service_account_path)
                .await
                .expect("[indexer gcs] Failed to read SERVICE_ACCOUNT credentials.");
            ClientConfig::default()
                .with_credentials(creds)
                .await
                .expect("[indexer gcs] Failed to create GCS client config.")
        },
        Err(_) => ClientConfig::default()
            .with_auth()
            .await
            .expect("[indexer gcs] Failed to create GCS client config."),
    };
    Client::new(config)
}

pub async fn get_file_store_metadata(bucket_name: String) -> FileStoreMetadata {
    let client = create_gcs_client().await;
    let metadata = client
        .download_object(
            &GetObjectRequest {
                bucket: bucket_name,
                object: METADATA_FILE_NAME.to_string(),
                ..Default::default()
            },
            &Range::default(),
        )
        .await
        .expect("[indexer gcs] Failed to get file store metadata.");

    serde_json::from_slice(&metadata).expect("Expected metadata to be valid JSON.")
}

/// Uploads the metadata to the bucket. If the metadata is not updated, the indexer will be restarted.
pub async fn upload_file_store_metadata(bucket_name: String, metadata: FileStoreMetadata) {
    let client = create_gcs_client().await;
    // If the metadata is not updated, the indexer will be restarted.
    client
        .upload_object(
            &UploadObjectRequest {
                bucket: bucket_name,
                ..Default::default()
            },
            serde_json::to_vec(&metadata).unwrap(),
            &UploadType::Simple(Media {
                name: METADATA_FILE_NAME.into(),
                content_type: JSON_FILE_TYPE.into(),
                content_length: None,
            }),
        )
        .await
        .unwrap();
}

#[derive(Serialize, Deserialize)]
pub struct TransactionsBlob {
    /// The version of the first transaction in the blob.
    pub starting_version: u64,
    /// The transactions in the blob.
    pub transactions: Vec<String>,
}

#[inline]
pub fn generate_blob_name(starting_version: u64) -> String {
    format!("{}/{}.json", BLOB_FOLDER_NAME, starting_version)
}

#[cfg(test)]
mod tests {
    #[test]
    fn verify_blob_naming() {
        assert_eq!(super::generate_blob_name(0), "blobs/0.json");
        assert_eq!(
            super::generate_blob_name(100_000_000),
            "blobs/100000000.json"
        );
        assert_eq!(
            super::generate_blob_name(1_000_000_000),
            "blobs/1000000000.json"
        );
        assert_eq!(
            super::generate_blob_name(10_000_000_000),
            "blobs/10000000000.json"
        );
        assert_eq!(
            super::generate_blob_name(u64::MAX),
            "blobs/18446744073709551615.json"
        );
    }
}
