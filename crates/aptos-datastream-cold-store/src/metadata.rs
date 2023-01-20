// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct BlobMetadata {
    /// The next version of the blob to process. This is the version of the first transaction in the next blob.
    pub version: u64,
    /// The chain id for verification.
    pub chain_id: u32,
    /// The chain name for verification.
    pub chain_name: String,
    /// The time of the last update. This is the time of the last blob written.
    /// This is used to determine if the blob is stale and triggers worker restart if yes.
    pub latest_update_time: u64,
}
