// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::U64;
use aptos_types::{chain_id::ChainId, ledger_info::LedgerInfoWithSignatures};
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};

/// The Ledger information representing the current state of the chain
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PoemObject)]
pub struct LedgerInfo {
    /// Chain ID of the current chain
    pub chain_id: u8,
    pub epoch: U64,
    pub ledger_version: U64,
    pub oldest_ledger_version: U64,
    pub block_height: U64,
    pub oldest_block_height: U64,
    pub ledger_timestamp: U64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub txn_encryption_key: Option<String>,
}

impl LedgerInfo {
    pub fn new(
        chain_id: &ChainId,
        info: &LedgerInfoWithSignatures,
        oldest_ledger_version: u64,
        oldest_block_height: u64,
        block_height: u64,
        txn_encryption_key: Option<String>,
    ) -> Self {
        let ledger_info = info.ledger_info();
        Self {
            chain_id: chain_id.id(),
            epoch: U64::from(ledger_info.epoch()),
            ledger_version: ledger_info.version().into(),
            oldest_ledger_version: oldest_ledger_version.into(),
            block_height: block_height.into(),
            oldest_block_height: oldest_block_height.into(),
            ledger_timestamp: ledger_info.timestamp_usecs().into(),
            txn_encryption_key,
        }
    }

    pub fn new_ledger_info(
        chain_id: &ChainId,
        epoch: u64,
        ledger_version: u64,
        oldest_ledger_version: u64,
        oldest_block_height: u64,
        block_height: u64,
        ledger_timestamp: u64,
        txn_encryption_key: Option<String>,
    ) -> Self {
        Self {
            chain_id: chain_id.id(),
            epoch: epoch.into(),
            ledger_version: ledger_version.into(),
            oldest_ledger_version: oldest_ledger_version.into(),
            block_height: block_height.into(),
            oldest_block_height: oldest_block_height.into(),
            ledger_timestamp: ledger_timestamp.into(),
            txn_encryption_key,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch.into()
    }

    pub fn version(&self) -> u64 {
        self.ledger_version.into()
    }

    pub fn oldest_version(&self) -> u64 {
        self.oldest_ledger_version.into()
    }

    pub fn timestamp(&self) -> u64 {
        self.ledger_timestamp.into()
    }
}
