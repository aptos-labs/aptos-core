// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::time::Duration;

pub const METADATA_FILE_NAME: &str = "metadata.json";

#[derive(Serialize, Deserialize, Debug)]
pub struct FileStoreMetadata {
    pub chain_id: u64,
    pub num_transactions_per_folder: u64,
    pub version: u64,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct FileMetadata {
    // [first_version, last_version)
    pub first_version: u64,
    pub last_version: u64,

    pub size_bytes: usize,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct BatchMetadata {
    pub files: Vec<FileMetadata>,
    pub suffix: Option<u64>,
}

#[async_trait::async_trait]
pub trait IFileStoreReader: Sync + Send {
    /// The tag of the store, for logging.
    fn tag(&self) -> &str;

    /// Returns true if the file store is initialized (non-empty).
    async fn is_initialized(&self) -> bool;

    async fn get_raw_file(&self, file_path: PathBuf) -> Result<Option<Vec<u8>>>;
}

#[async_trait::async_trait]
pub trait IFileStoreWriter: Sync + Send {
    async fn save_raw_file(&self, file_path: PathBuf, data: Vec<u8>) -> Result<()>;

    fn max_update_frequency(&self) -> Duration;
}

#[async_trait::async_trait]
pub trait IFileStore: IFileStoreReader + IFileStoreWriter {}

impl<T> IFileStore for T where T: IFileStoreReader + IFileStoreWriter {}
