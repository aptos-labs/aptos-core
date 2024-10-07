// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::parsed_transaction_output::TransactionsWithParsedOutput;
use anyhow::{ensure, Result};
use aptos_storage_interface::{cached_state_view::StateCache, state_delta::StateDelta};
use aptos_types::{
    epoch_state::EpochState,
    ledger_info::LedgerInfo,
    transaction::{BlockEndInfo, TransactionStatus, Version},
};

// FIXME(aldenhu): check debug impls
// FIXME(aldenh): proper naming
#[derive(Debug)]
pub struct ChunkOutput {
    pub first_version: Version,
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
    pub fn new_empty_at_ledger_info(parent_state: &StateDelta, ledger_info: &LedgerInfo) -> Self {
        assert!(parent_state.current_version == Some(ledger_info.version()),);
        Self {
            first_version: parent_state.next_version(),
            statuses_for_input_txns: vec![],
            to_commit: TransactionsWithParsedOutput::new_empty(),
            to_discard: TransactionsWithParsedOutput::new_empty(),
            to_retry: TransactionsWithParsedOutput::new_empty(),
            state_cache: StateCache::new_empty(&parent_state.current),
            block_end_info: None,
            next_epoch_state: ledger_info.next_epoch_state().cloned(),
        }
    }

    pub fn new_empty_following_this(&self) -> Self {
        Self {
            first_version: self.next_version(),
            statuses_for_input_txns: vec![],
            to_commit: TransactionsWithParsedOutput::new_empty(),
            to_discard: TransactionsWithParsedOutput::new_empty(),
            to_retry: TransactionsWithParsedOutput::new_empty(),
            state_cache: StateCache::new_empty(&self.state_cache.frozen_base.smt),
            block_end_info: None,
            next_epoch_state: self.next_epoch_state.clone(),
        }
    }

    pub fn next_version(&self) -> Version {
        self.first_version + self.to_commit.len() as Version
    }

    pub fn ends_epoch(&self) -> bool {
        self.next_epoch_state.is_some()
    }

    pub fn ensure_is_block(&self) -> Result<()> {
        if self.ends_epoch() {
            ensure!(self.to_commit.is_empty() || self.to_commit.ends_with_reconfig());
        } else {
            ensure!(!self.to_commit.is_empty());
            ensure!(self.to_commit.ends_with_state_checkpoint());
        }

        Ok(())
    }

    pub fn ensure_is_replayed(&self) -> Result<()> {
        ensure!(self.to_discard.is_empty());
        ensure!(self.to_retry.is_empty());

        Ok(())
    }

    pub fn check_aborts_discards_retries(
        &self,
        allow_aborts: bool,
        allow_discards: bool,
        allow_retries: bool,
    ) {
        let aborts = self
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
            .to_discard
            .iter()
            .take(3)
            .map(|(txn, output)| format!("{:?}: {:?}", txn, output.status()))
            .collect::<Vec<_>>();
        let retries_3 = self
            .to_retry
            .iter()
            .take(3)
            .map(|(txn, output)| format!("{:?}: {:?}", txn, output.status()))
            .collect::<Vec<_>>();

        if !aborts.is_empty() || !discards_3.is_empty() || !retries_3.is_empty() {
            println!(
                "Some transactions were not successful: {} aborts, {} discards and {} retries out of {}, examples: aborts: {:?}, discards: {:?}, retries: {:?}",
                aborts.len(),
                self.to_discard.len(),
                self.to_retry.len(),
                self.statuses_for_input_txns.len(),
                &aborts[..aborts.len().min(3)],
                discards_3,
                retries_3,
            )
        }

        assert!(
            allow_aborts || aborts.is_empty(),
            "No aborts allowed, {}, examples: {:?}",
            aborts.len(),
            &aborts[..aborts.len().min(3)]
        );
        assert!(
            allow_discards || discards_3.is_empty(),
            "No discards allowed, {}, examples: {:?}",
            self.to_discard.len(),
            discards_3,
        );
        assert!(
            allow_retries || retries_3.is_empty(),
            "No retries allowed, {}, examples: {:?}",
            self.to_retry.len(),
            retries_3,
        );
    }
}
