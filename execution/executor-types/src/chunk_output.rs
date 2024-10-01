// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::ensure;
use anyhow::Result;
use aptos_storage_interface::cached_state_view::StateCache;
use aptos_types::epoch_state::EpochState;
use aptos_types::transaction::{BlockEndInfo, TransactionStatus};
use crate::parsed_transaction_output::TransactionsWithParsedOutput;

// FIXME(aldenhu): check debug impls
#[derive(Debug)]
pub struct ChunkOutput {
    // Statuses of the input transactions, in the same order as the input transactions.
    // Contains BlockMetadata/Validator transactions,
    // but doesn't contain StateCheckpoint/BlockEpilogue, as those get added during execution
    pub statuses_for_input_txns: Vec<TransactionStatus>,
    // List of all transactions to be committed, including StateCheckpoint/BlockEpilogue if needed.
    pub to_commit: TransactionsWithParsedOutput,
    pub to_discard: TransactionsWithParsedOutput,
    pub to_retry: TransactionsWithParsedOutput,

    /// Carries the frozen base state view, so all in-mem nodes involved won't drop before the
    /// execution result is processed; as well as all the accounts touched during execution, together
    /// with their proofs.
    pub state_cache: StateCache,
    /// Optional StateCheckpoint payload
    pub block_end_info: Option<BlockEndInfo>,
    /// Optional EpochState payload.
    /// Only present if the block is the last block of an epoch, and is parsed output of the
    /// state cache.
    pub next_epoch_state: Option<EpochState>,
}

impl ChunkOutput {
    pub fn ensure_is_block(&self) -> Result<()> {
        self.to_commit.ends_epoch()
    }

    pub fn ensure_is_replay(&self) -> Result<()> {

    }
}
