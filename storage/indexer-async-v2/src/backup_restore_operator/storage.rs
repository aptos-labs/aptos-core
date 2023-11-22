// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use fs_extra::dir::{self, CopyOptions};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct BackupRestoreMetadata {
    pub chain_id: u64,
    pub epoch: u64,
}

pub fn copy_directory(src: &str, dst: &str) -> Result<(), Box<dyn std::error::Error>> {
    let options = CopyOptions::new();
    dir::copy(src, dst, &options)?;
    Ok(())
}
