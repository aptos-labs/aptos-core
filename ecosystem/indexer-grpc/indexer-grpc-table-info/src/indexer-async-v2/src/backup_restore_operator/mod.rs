// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

pub mod gcs;

pub const FILE_FOLDER_NAME: &str = "files";
pub const METADATA_FILE_NAME: &str = "metadata.json";
pub const JSON_FILE_TYPE: &str = "application/json";
pub const TAR_FILE_TYPE: &str = "application/gzip";
pub const TEMPORARY_MARKER: u64 = u64::MAX; // A value that represents an ongoing update

#[inline]
pub fn generate_blob_name(epoch: u64) -> String {
    format!("{}/{}.tar.gz", FILE_FOLDER_NAME, epoch)
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct BackupRestoreMetadata {
    pub chain_id: u64,
    pub epoch: u64,
    pub version: u64,
}

impl BackupRestoreMetadata {
    pub fn new(chain_id: u64, epoch: u64, version: u64) -> Self {
        Self {
            chain_id,
            epoch,
            version,
        }
    }
}
