// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    cached_state_view::ShardedStateCache,
    sharded_state_update_refs::ShardedStateUpdateRefs,
    state_authenticator::StateAuthenticator,
    state_delta::{InMemState},
};
use aptos_types::{
    transaction::{Transaction, TransactionInfo, TransactionOutput, Version},
};

/// FIXME(aldenhu): clean up unused fields
#[derive(Clone)]
pub struct ChunkToCommit<'a> {
    pub first_version: Version,
    pub transactions: &'a [Transaction],
    pub transaction_outputs: &'a [TransactionOutput],
    pub transaction_infos: &'a [TransactionInfo],
    pub base_state_version: Option<Version>,
    pub state_update_refs: &'a ShardedStateUpdateRefs<'a>,
    pub parent_state: &'a InMemState,
    pub state: &'a InMemState,
    pub parent_auth: &'a StateAuthenticator,
    pub state_auth: &'a StateAuthenticator,
    pub last_checkpoint_state: Option<&'a InMemState>,
    pub last_checkpoint_auth: Option<&'a StateAuthenticator>,
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
