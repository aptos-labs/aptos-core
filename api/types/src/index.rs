// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{LedgerInfo, U64};
use aptos_config::config::RoleType;
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};

// The data in IndexResponse is flattened into a single JSON map to offer
// easier parsing for clients.

/// The struct holding all data returned to the client by the
/// index endpoint (i.e., GET "/").
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PoemObject, Serialize)]
pub struct IndexResponse {
    /// Chain ID of the current chain
    pub chain_id: u8,
    /// Current epoch of the chain
    pub epoch: U64,
    /// Current ledger version of the chain
    pub ledger_version: U64,
    /// Oldest non-pruned ledger version of the chain
    pub oldest_ledger_version: U64,
    /// Current timestamp of the chain
    pub ledger_timestamp: U64,
    /// Role of the node `Validator` or `FullNode`
    pub node_role: RoleType,
    /// Oldest non-pruned block height of the chain
    pub oldest_block_height: U64,
    /// Current block height of the chain
    pub block_height: U64,
}

impl IndexResponse {
    pub fn new(ledger_info: LedgerInfo, node_role: RoleType) -> IndexResponse {
        Self {
            chain_id: ledger_info.chain_id,
            epoch: ledger_info.epoch,
            ledger_version: ledger_info.ledger_version,
            oldest_ledger_version: ledger_info.oldest_ledger_version,
            ledger_timestamp: ledger_info.ledger_timestamp,
            oldest_block_height: ledger_info.oldest_block_height,
            block_height: ledger_info.block_height,
            node_role,
        }
    }
}
