// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::U64;

use diem_types::{chain_id::ChainId, ledger_info::LedgerInfoWithSignatures};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct LedgerInfo {
    pub chain_id: u8,
    pub ledger_version: U64,
    pub ledger_timestamp: U64,
}

impl LedgerInfo {
    pub fn new(chain_id: &ChainId, info: &LedgerInfoWithSignatures) -> Self {
        let ledger_info = info.ledger_info();
        Self {
            chain_id: chain_id.id(),
            ledger_version: ledger_info.version().into(),
            ledger_timestamp: ledger_info.timestamp_usecs().into(),
        }
    }

    pub fn version(&self) -> u64 {
        self.ledger_version.into()
    }

    pub fn timestamp(&self) -> u64 {
        self.ledger_timestamp.into()
    }
}
