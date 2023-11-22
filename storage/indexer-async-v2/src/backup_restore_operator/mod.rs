// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_storage_interface::DbWriter;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

pub mod gcs;
pub use gcs::*;
pub mod local;
pub use local::*;
pub mod storage;

pub const FILE_FOLDER_NAME: &str = "files";
const METADATA_FILE_NAME: &str = "metadata.json";

#[inline]
pub fn generate_blob_name(epoch: u64) -> String {
    format!("{}/{}.tar.gz", FILE_FOLDER_NAME, epoch)
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct BackupRestoreMetadata {
    pub chain_id: u64,
    pub epoch: u64,
}

impl BackupRestoreMetadata {
    pub fn new(chain_id: u64, epoch: u64) -> Self {
        Self { chain_id, epoch }
    }
}

#[async_trait::async_trait]
pub trait BackupRestoreOperator: Send + Sync {
    fn get_metadata_epoch(&self) -> u64;
    fn set_metadata_epoch(&self, epoch: u64);
    async fn verify_storage_bucket_existence(&self);
    async fn get_metadata(&self) -> Option<BackupRestoreMetadata>;
    async fn create_default_metadata_if_absent(
        &self,
        expected_chain_id: u64,
    ) -> anyhow::Result<BackupRestoreMetadata>;
    async fn update_metadata(&self, chain_id: u64, epoch: u64) -> anyhow::Result<()>;
    async fn upload_snapshot(
        &self,
        chain_id: u64,
        epoch: u64,
        snapshot_path: PathBuf,
    ) -> anyhow::Result<()>;
    async fn try_upload_snapshot(
        &self,
        chain_id: u64,
        epoch: u64,
        db_writer: Arc<dyn DbWriter>,
        snapshot_path: PathBuf,
    ) -> anyhow::Result<()>;
    async fn restore_snapshot(&self, chain_id: u64, db_path: PathBuf) -> anyhow::Result<()>;
}
