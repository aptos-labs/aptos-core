// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::U64;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct LedgerInfo {
    pub chain_id: u8,
    pub ledger_version: U64,
    pub ledger_timestamp: U64,
}
