// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct LedgerInfo {
    pub chain_id: u8,
    #[serde_as(as = "DisplayFromStr")]
    pub ledger_version: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub ledger_timestamp: u64,
}
