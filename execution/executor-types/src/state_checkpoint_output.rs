// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::iter::repeat;
use crate::parsed_transaction_output::TransactionsWithParsedOutput;
use anyhow::{ensure, Result};
use aptos_crypto::HashValue;
use aptos_storage_interface::cached_state_view::ShardedStateCache;
use aptos_types::{
    state_store::ShardedStateUpdates,
    transaction::{block_epilogue::BlockEndInfo, TransactionStatus},
};
use itertools::zip_eq;
use aptos_types::transaction::{BlockEpiloguePayload, ExecutionStatus, Transaction, TransactionAuxiliaryData, TransactionOutput};
use log::error;
use aptos_types::write_set::WriteSet;
use crate::ParsedTransactionOutput;

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
    pub fn parse(
        mut transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        append_state_checkpoint_to_block: Option<HashValue>,
        block_end_info: Option<BlockEndInfo>,
    ) -> Self {
        let mut transaction_outputs: Vec<ParsedTransactionOutput> =
            transaction_outputs.into_iter().map(Into::into).collect();
        // N.B. off-by-1 intentionally, for exclusive index
        let new_epoch_marker = transaction_outputs
            .iter()
            .position(|o| o.is_reconfig())
            .map(|idx| idx + 1);

        let block_gas_limit_marker = transaction_outputs
            .iter()
            .position(|o| matches!(o.status(), TransactionStatus::Retry));

        // Transactions after the epoch ending txn are all to be retried.
        // Transactions after the txn that exceeded per-block gas limit are also to be retried.
        let to_retry = if let Some(pos) = new_epoch_marker {
            TransactionsWithParsedOutput::new(
                transactions.drain(pos..).collect(),
                transaction_outputs.drain(pos..).collect(),
            )
        } else if let Some(pos) = block_gas_limit_marker {
            TransactionsWithParsedOutput::new(
                transactions.drain(pos..).collect(),
                transaction_outputs.drain(pos..).collect(),
            )
        } else {
            TransactionsWithParsedOutput::new_empty()
        };

        let state_checkpoint_to_add =
            new_epoch_marker.map_or_else(|| append_state_checkpoint_to_block, |_| None);

        let keeps_and_discards = transaction_outputs.iter().map(|t| t.status()).cloned();
        let retries = repeat(TransactionStatus::Retry).take(to_retry.len());

        let status = keeps_and_discards.chain(retries).collect();

        let to_discard = {
            let mut res = TransactionsWithParsedOutput::new_empty();
            for idx in 0..transactions.len() {
                if transaction_outputs[idx].status().is_discarded() {
                    res.push(transactions[idx].clone(), transaction_outputs[idx].clone());
                } else if !res.is_empty() {
                    transactions[idx - res.len()] = transactions[idx].clone();
                    transaction_outputs[idx - res.len()] = transaction_outputs[idx].clone();
                }
            }
            if !res.is_empty() {
                let remaining = transactions.len() - res.len();
                transactions.truncate(remaining);
                transaction_outputs.truncate(remaining);
            }
            res
        };
        let to_commit = {
            let mut res = TransactionsWithParsedOutput::new(transactions, transaction_outputs);

            // Append the StateCheckpoint transaction to the end of to_keep
            if let Some(block_id) = state_checkpoint_to_add {
                let state_checkpoint_txn = block_end_info.map_or(
                    Transaction::StateCheckpoint(block_id),
                    |block_end_info| {
                        Transaction::BlockEpilogue(BlockEpiloguePayload::V0 {
                            block_id,
                            block_end_info,
                        })
                    },
                );
                let state_checkpoint_txn_output: ParsedTransactionOutput =
                    Into::into(TransactionOutput::new(
                        WriteSet::default(),
                        Vec::new(),
                        0,
                        TransactionStatus::Keep(ExecutionStatus::Success),
                        TransactionAuxiliaryData::default(),
                    ));
                res.push(state_checkpoint_txn, state_checkpoint_txn_output);
            }
            res
        };

        // Sanity check transactions with the Discard status:
        to_discard.iter().for_each(|(t, o)| {
            // In case a new status other than Retry, Keep and Discard is added:
            if !matches!(o.status(), TransactionStatus::Discard(_)) {
                error!("Status other than Retry, Keep or Discard; Transaction discarded.");
            }
            // VM shouldn't have output anything for discarded transactions, log if it did.
            if !o.write_set().is_empty() || !o.events().is_empty() {
                error!(
                    "Discarded transaction has non-empty write set or events. \
                        Transaction: {:?}. Status: {:?}.",
                    t,
                    o.status(),
                );
            }
        });

        Self {
            statuses_for_input_txns: status,
            to_commit,
            to_discard,
            to_retry,
        }
    }

    pub fn to_commit(&self) -> &TransactionsWithParsedOutput {
        &self.to_commit
    }

    pub fn ends_epoch(&self) -> bool {
        self.to_commit.parsed_outputs().last().map_or(false, |o| o.is_reconfig())
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
