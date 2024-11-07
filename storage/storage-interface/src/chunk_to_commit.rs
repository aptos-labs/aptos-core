// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{cached_state_view::ShardedStateCache, state_delta::StateDelta};
use aptos_types::{
    state_store::ShardedStateUpdates,
    transaction::{Transaction, TransactionInfo, TransactionOutput, Version},
};

#[derive(Clone)]
pub struct ChunkToCommit<'a> {
    pub first_version: Version,
    pub transactions: &'a [Transaction],
    // TODO(aldenhu): make it a ref
    pub transaction_outputs: &'a [TransactionOutput],
    pub transaction_infos: &'a [TransactionInfo],
    pub base_state_version: Option<Version>,
    pub latest_in_memory_state: &'a StateDelta,
    pub per_version_state_updates: &'a [ShardedStateUpdates],
    pub state_updates_until_last_checkpoint: Option<&'a ShardedStateUpdates>,
    pub sharded_state_cache: Option<&'a ShardedStateCache>,
    pub is_reconfig: bool,
}

impl<'a> ChunkToCommit<'a> {
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn next_version(&self) -> Version {
        self.first_version + self.len() as Version
    }

    pub fn expect_last_version(&self) -> Version {
        self.next_version() - 1
    }
}
