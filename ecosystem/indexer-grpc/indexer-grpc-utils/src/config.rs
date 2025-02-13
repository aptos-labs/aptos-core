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
    #[serde(default = "default_enable_compression")]
    pub enable_compression: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalFileStore {
    pub local_file_store_path: PathBuf,
    #[serde(default = "default_enable_compression")]
    pub enable_compression: bool,
}

const fn default_enable_compression() -> bool {
    false
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "file_store_type")]
pub enum IndexerGrpcFileStoreConfig {
    LocalFileStore(LocalFileStore),
}

impl Default for IndexerGrpcFileStoreConfig {
    fn default() -> Self {
        IndexerGrpcFileStoreConfig::LocalFileStore(LocalFileStore {
            local_file_store_path: std::env::current_dir().unwrap(),
            enable_compression: false,
        })
    }
}

impl IndexerGrpcFileStoreConfig {
    pub fn create(&self) -> Box<dyn crate::file_store_operator::FileStoreOperator> {
        match self {
            IndexerGrpcFileStoreConfig::LocalFileStore(local_file_store) => Box::new(
                crate::file_store_operator::local::LocalFileStoreOperator::new(
                    local_file_store.local_file_store_path.clone(),
                    local_file_store.enable_compression,
                ),
            ),
        }
    }
}
