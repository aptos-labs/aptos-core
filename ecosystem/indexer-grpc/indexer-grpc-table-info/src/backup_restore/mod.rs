// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

pub mod fs_ops;
pub mod gcs;

pub const FILE_FOLDER_NAME: &str = "files";
pub const METADATA_FILE_NAME: &str = "metadata.json";
pub const JSON_FILE_TYPE: &str = "application/json";
pub const TAR_FILE_TYPE: &str = "application/gzip";

#[inline]
pub fn generate_blob_name(chain_id: u64, epoch: u64) -> String {
    format!(
        "{}/chain_id_{}_epoch_{}.tar.gz",
        FILE_FOLDER_NAME, chain_id, epoch
    )
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

impl From<Vec<u8>> for BackupRestoreMetadata {
    fn from(bytes: Vec<u8>) -> Self {
        serde_json::from_slice(bytes.as_slice())
            .expect("Failed to deserialize BackupRestoreMetadata file.")
    }
}
