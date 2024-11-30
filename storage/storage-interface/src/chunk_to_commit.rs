// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{
    sharded_state_update_refs::ShardedStateUpdateRefs, state::State, state_summary::StateSummary,
    state_view::cached_state_view::ShardedStateCache,
};
use aptos_types::transaction::{Transaction, TransactionInfo, TransactionOutput, Version};

#[derive(Clone)]
pub struct ChunkToCommit<'a> {
    pub first_version: Version,
    pub transactions: &'a [Transaction],
    pub transaction_outputs: &'a [TransactionOutput],
    pub transaction_infos: &'a [TransactionInfo],
    pub last_state_checkpoint: Option<&'a State>,
    pub state: &'a State,
    pub last_state_checkpoint_summary: Option<&'a StateSummary>,
    pub state_summary: &'a StateSummary,
    pub state_update_refs: &'a ShardedStateUpdateRefs<'a>,
    pub state_reads: Option<&'a ShardedStateCache>,
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
