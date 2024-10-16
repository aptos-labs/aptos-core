// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::parsed_transaction_output::TransactionsWithParsedOutput;
use anyhow::{ensure, Result};
use aptos_crypto::HashValue;
use aptos_storage_interface::cached_state_view::ShardedStateCache;
use aptos_types::{
    state_store::ShardedStateUpdates,
    transaction::{block_epilogue::BlockEndInfo, TransactionStatus},
};
use itertools::zip_eq;

#[derive(Default)]
pub struct TransactionsByStatus {
    // Statuses of the input transactions, in the same order as the input transactions.
    // Contains BlockMetadata/Validator transactions,
    // but doesn't contain StateCheckpoint/BlockEpilogue, as those get added during execution
    statuses_for_input_txns: Vec<TransactionStatus>,
    // List of all transactions to be committed, including StateCheckpoint/BlockEpilogue if needed.
    to_commit: TransactionsWithParsedOutput,
    to_discard: TransactionsWithParsedOutput,
    to_retry: TransactionsWithParsedOutput,
}

impl TransactionsByStatus {
    pub fn new(
        statuses_for_input_txns: Vec<TransactionStatus>,
        to_commit: TransactionsWithParsedOutput,
        to_discard: TransactionsWithParsedOutput,
        to_retry: TransactionsWithParsedOutput,
    ) -> Self {
        Self {
            statuses_for_input_txns,
            to_commit,
            to_discard,
            to_retry,
        }
    }

    pub fn input_txns_len(&self) -> usize {
        self.statuses_for_input_txns.len()
    }

    pub fn into_inner(
        self,
    ) -> (
        Vec<TransactionStatus>,
        TransactionsWithParsedOutput,
        TransactionsWithParsedOutput,
        TransactionsWithParsedOutput,
    ) {
        (
            self.statuses_for_input_txns,
            self.to_commit,
            self.to_discard,
            self.to_retry,
        )
    }
}

#[derive(Default)]
pub struct StateCheckpointOutput {
    txns: TransactionsByStatus,
    per_version_state_updates: Vec<ShardedStateUpdates>,
    state_checkpoint_hashes: Vec<Option<HashValue>>,
    state_updates_before_last_checkpoint: Option<ShardedStateUpdates>,
    sharded_state_cache: ShardedStateCache,
    block_end_info: Option<BlockEndInfo>,
}

impl StateCheckpointOutput {
    pub fn new(
        txns: TransactionsByStatus,
        per_version_state_updates: Vec<ShardedStateUpdates>,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
        state_updates_before_last_checkpoint: Option<ShardedStateUpdates>,
        sharded_state_cache: ShardedStateCache,
        block_end_info: Option<BlockEndInfo>,
    ) -> Self {
        Self {
            txns,
            per_version_state_updates,
            state_checkpoint_hashes,
            state_updates_before_last_checkpoint,
            sharded_state_cache,
            block_end_info,
        }
    }

    pub fn input_txns_len(&self) -> usize {
        self.txns.input_txns_len()
    }

    pub fn txns_to_commit_len(&self) -> usize {
        self.txns.to_commit.len()
    }

    pub fn into_inner(
        self,
    ) -> (
        TransactionsByStatus,
        Vec<ShardedStateUpdates>,
        Vec<Option<HashValue>>,
        Option<ShardedStateUpdates>,
        ShardedStateCache,
        Option<BlockEndInfo>,
    ) {
        (
            self.txns,
            self.per_version_state_updates,
            self.state_checkpoint_hashes,
            self.state_updates_before_last_checkpoint,
            self.sharded_state_cache,
            self.block_end_info,
        )
    }

    pub fn check_and_update_state_checkpoint_hashes(
        &mut self,
        trusted_hashes: Vec<Option<HashValue>>,
    ) -> Result<()> {
        let len = self.state_checkpoint_hashes.len();
        ensure!(
            len == trusted_hashes.len(),
            "Number of txns doesn't match. self: {len}, trusted: {}",
            trusted_hashes.len()
        );

        zip_eq(
            self.state_checkpoint_hashes.iter_mut(),
            trusted_hashes.iter(),
        )
        .try_for_each(|(self_hash, trusted_hash)| {
            if self_hash.is_none() && trusted_hash.is_some() {
                *self_hash = *trusted_hash;
            } else {
                ensure!(self_hash == trusted_hash,
                        "State checkpoint hash doesn't match, self: {self_hash:?}, trusted: {trusted_hash:?}");
            }
            Ok(())
        })
    }

    pub fn check_aborts_discards_retries(
        &self,
        allow_aborts: bool,
        allow_discards: bool,
        allow_retries: bool,
    ) {
        let aborts = self
            .txns
            .to_commit
            .iter()
            .flat_map(|(txn, output)| match output.status().status() {
                Ok(execution_status) => {
                    if execution_status.is_success() {
                        None
                    } else {
                        Some(format!("{:?}: {:?}", txn, output.status()))
                    }
                },
                Err(_) => None,
            })
            .collect::<Vec<_>>();

        let discards_3 = self
            .txns
            .to_discard
            .iter()
            .take(3)
            .map(|(txn, output)| format!("{:?}: {:?}", txn, output.status()))
            .collect::<Vec<_>>();
        let retries_3 = self
            .txns
            .to_retry
            .iter()
            .take(3)
            .map(|(txn, output)| format!("{:?}: {:?}", txn, output.status()))
            .collect::<Vec<_>>();

        if !aborts.is_empty() || !discards_3.is_empty() || !retries_3.is_empty() {
            println!(
                "Some transactions were not successful: {} aborts, {} discards and {} retries out of {}, examples: aborts: {:?}, discards: {:?}, retries: {:?}",
                aborts.len(),
                self.txns.to_discard.len(),
                self.txns.to_retry.len(),
                self.input_txns_len(),
                &aborts[..(aborts.len().min(3))],
                discards_3,
                retries_3,
            )
        }

        assert!(
            allow_aborts || aborts.is_empty(),
            "No aborts allowed, {}, examples: {:?}",
            aborts.len(),
            &aborts[..(aborts.len().min(3))]
        );
        assert!(
            allow_discards || discards_3.is_empty(),
            "No discards allowed, {}, examples: {:?}",
            self.txns.to_discard.len(),
            discards_3,
        );
        assert!(
            allow_retries || retries_3.is_empty(),
            "No retries allowed, {}, examples: {:?}",
            self.txns.to_retry.len(),
            retries_3,
        );
    }
}
