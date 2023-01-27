// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use cloud_storage::Object;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct FileStoreMetadata {
    /// The chain_id for the file store.
    pub chain_id: u64,

    /// The blob size for the file store.
    pub blob_size: u64,

    /// Current version of the file store.
    pub version: u64,
}

pub async fn get_file_store_metadata(bucket_name: String) -> FileStoreMetadata {
    let metadata = Object::download(&bucket_name, "metadata.json")
        .await
        .expect("[indexer gcs] Failed to get file store metadata.");

    serde_json::from_slice(&metadata).expect("Expected metadata to be valid JSON.")
}
