// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::HashValue;
use serde::{Deserialize, Serialize};

/// A description of a block
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub block_height: u64,
    pub block_hash: HashValue,
    pub block_timestamp: u64,
    pub start_version: u64,
    pub end_version: u64,
    pub num_transactions: u16,
}
