// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_logger::Schema;
use aptos_types::{state_store::StateViewId, transaction::Version};
use serde::Serialize;

#[derive(Schema, Clone)]
pub struct AdapterLogSchema {
    name: LogEntry,

    // only one of the next 3 `Option`s will be set. Unless it is in testing mode
    // in which case nothing will be set.
    // Those values are coming from `StateView::id()` and the info carried by
    // `StateViewId`

    // StateViewId::BlockExecution - typical transaction execution
    block_id: Option<HashValue>,
    // StateViewId::ChunkExecution - state sync
    first_version: Option<Version>,
    // StateViewId::TransactionValidation - validation
    base_version: Option<Version>,

    // transaction position in the list of transactions in the block,
    // 0 if the transaction is not part of a block (i.e. validation).
    txn_idx: usize,
}

impl AdapterLogSchema {
    pub fn new(view_id: StateViewId, txn_idx: usize) -> Self {
        match view_id {
            StateViewId::BlockExecution { block_id } => Self {
                name: LogEntry::Execution,
                block_id: Some(block_id),
                first_version: None,
                base_version: None,
                txn_idx,
            },
            StateViewId::GetReadyScheduledTxns {
                block_id,
                child_block_id: _,
            } => Self {
                name: LogEntry::Execution,
                block_id: Some(block_id),
                first_version: None,
                base_version: None,
                txn_idx,
            },
            StateViewId::ChunkExecution { first_version } => Self {
                name: LogEntry::Execution,
                block_id: None,
                first_version: Some(first_version),
                base_version: None,
                txn_idx,
            },
            StateViewId::TransactionValidation { base_version } => Self {
                name: LogEntry::Validation,
                block_id: None,
                first_version: None,
                base_version: Some(base_version),
                txn_idx,
            },
            StateViewId::Replay => Self {
                name: LogEntry::Execution,
                block_id: None,
                first_version: None,
                base_version: None,
                txn_idx,
            },
            StateViewId::Miscellaneous => Self {
                name: LogEntry::Miscellaneous,
                block_id: None,
                first_version: None,
                base_version: None,
                txn_idx,
            },
        }
    }

    // Is the adapter log schema used in a context that supports speculative
    // logging (block execution and state-sync). It is from the name.
    pub(crate) fn speculation_supported(&self) -> bool {
        matches!(self.name, LogEntry::Execution)
    }

    pub(crate) fn get_txn_idx(&self) -> usize {
        self.txn_idx
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEntry {
    Execution,
    Validation,
    Miscellaneous, // usually testing
}
