// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
/// Common configuration for Indexer GRPC Store.
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GcsFileStore {
    pub gcs_file_store_bucket_name: String,
    // Required to operate on GCS.
    pub gcs_file_store_service_account_key_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalFileStore {
    pub local_file_store_path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "file_store_type")]
pub enum IndexerGrpcFileStoreConfig {
    GcsFileStore(GcsFileStore),
    LocalFileStore(LocalFileStore),
}

impl Default for IndexerGrpcFileStoreConfig {
    fn default() -> Self {
        IndexerGrpcFileStoreConfig::LocalFileStore(LocalFileStore {
            local_file_store_path: std::env::current_dir().unwrap(),
        })
    }
}
