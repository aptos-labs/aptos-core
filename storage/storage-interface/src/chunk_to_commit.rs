// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{
    sharded_state_update_refs::ShardedStateUpdateRefs, sharded_state_updates::ShardedStateUpdates,
    state_delta::StateDelta, state_view::cached_state_view::ShardedStateCache,
};
use aptos_types::transaction::{Transaction, TransactionInfo, TransactionOutput, Version};

#[derive(Clone)]
pub struct ChunkToCommit<'a> {
    pub first_version: Version,
    pub transactions: &'a [Transaction],
    pub transaction_outputs: &'a [TransactionOutput],
    pub transaction_infos: &'a [TransactionInfo],
    pub base_state_version: Option<Version>,
    pub latest_in_memory_state: &'a StateDelta,
    pub state_update_refs: &'a ShardedStateUpdateRefs<'a>,
    pub state_updates_until_last_checkpoint: Option<&'a ShardedStateUpdates>,
    pub sharded_state_cache: Option<&'a ShardedStateCache>,
    pub is_reconfig: bool,
}

impl ChunkToCommit<'_> {
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
