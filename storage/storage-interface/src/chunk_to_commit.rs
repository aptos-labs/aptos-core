// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{cached_state_view::ShardedStateCache, state_delta::StateDelta};
use aptos_types::{
    state_store::ShardedStateUpdates,
    transaction::{TransactionToCommit, Version},
};

#[derive(Copy, Clone)]
pub struct ChunkToCommit<'a> {
    pub first_version: Version,
    pub base_state_version: Option<Version>,
    pub txns_to_commit: &'a [TransactionToCommit],
    pub latest_in_memory_state: &'a StateDelta,
    pub state_updates_until_last_checkpoint: Option<&'a ShardedStateUpdates>,
    pub sharded_state_cache: Option<&'a ShardedStateCache>,
}
