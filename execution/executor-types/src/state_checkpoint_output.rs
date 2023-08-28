// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::ParsedTransactionOutput;
use aptos_crypto::HashValue;
use aptos_storage_interface::cached_state_view::ShardedStateCache;
use aptos_types::{
    state_store::ShardedStateUpdates,
    transaction::{Transaction, TransactionStatus},
};

#[derive(Default)]
pub struct TransactionsByStatus {
    statuses: Vec<TransactionStatus>,
    to_keep: Vec<(Transaction, ParsedTransactionOutput)>,
    to_discard: Vec<Transaction>,
    to_retry: Vec<Transaction>,
}

impl TransactionsByStatus {
    pub fn new(
        status: Vec<TransactionStatus>,
        to_keep: Vec<(Transaction, ParsedTransactionOutput)>,
        to_discard: Vec<Transaction>,
        to_retry: Vec<Transaction>,
    ) -> Self {
        Self {
            statuses: status,
            to_keep,
            to_discard,
            to_retry,
        }
    }

    pub fn num_txns_to_keep(&self) -> usize {
        self.to_keep.len()
    }

    pub fn txn_statuses(&self) -> &[TransactionStatus] {
        &self.statuses
    }

    pub fn into_inner(
        self,
    ) -> (
        Vec<TransactionStatus>,
        Vec<(Transaction, ParsedTransactionOutput)>,
        Vec<Transaction>,
        Vec<Transaction>,
    ) {
        (self.statuses, self.to_keep, self.to_discard, self.to_retry)
    }
}

#[derive(Default)]
pub struct StateCheckpointOutput {
    txns: TransactionsByStatus,
    state_updates_vec: Vec<ShardedStateUpdates>,
    state_checkpoint_hashes: Vec<Option<HashValue>>,
    block_state_updates: ShardedStateUpdates,
    sharded_state_cache: ShardedStateCache,
}

impl StateCheckpointOutput {
    pub fn new(
        txns: TransactionsByStatus,
        state_updates_vec: Vec<ShardedStateUpdates>,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
        block_state_updates: ShardedStateUpdates,
        sharded_state_cache: ShardedStateCache,
    ) -> Self {
        Self {
            txns,
            state_updates_vec,
            state_checkpoint_hashes,
            block_state_updates,
            sharded_state_cache,
        }
    }

    pub fn txn_statuses(&self) -> &[TransactionStatus] {
        self.txns.txn_statuses()
    }

    pub fn into_inner(
        self,
    ) -> (
        TransactionsByStatus,
        Vec<ShardedStateUpdates>,
        Vec<Option<HashValue>>,
        ShardedStateUpdates,
        ShardedStateCache,
    ) {
        (
            self.txns,
            self.state_updates_vec,
            self.state_checkpoint_hashes,
            self.block_state_updates,
            self.sharded_state_cache,
        )
    }
}
