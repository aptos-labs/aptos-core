// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    planned::Planned,
    transactions_with_output::{TransactionsToKeep, TransactionsWithOutput},
};
use aptos_drop_helper::DropHelper;
use aptos_storage_interface::state_store::{
    state::LedgerState, state_view::cached_state_view::ShardedStateCache,
};
use aptos_types::{
    contract_event::ContractEvent,
    epoch_state::EpochState,
    transaction::{
        block_epilogue::BlockEndInfo, ExecutionStatus, Transaction, TransactionStatus, Version,
    },
};
use derive_more::Deref;
use std::sync::Arc;

#[derive(Clone, Debug, Deref)]
pub struct ExecutionOutput {
    #[deref]
    inner: Arc<DropHelper<Inner>>,
}

impl ExecutionOutput {
    pub fn new(
        is_block: bool,
        first_version: Version,
        statuses_for_input_txns: Vec<TransactionStatus>,
        to_commit: TransactionsToKeep,
        to_discard: TransactionsWithOutput,
        to_retry: TransactionsWithOutput,
        result_state: LedgerState,
        state_reads: ShardedStateCache,
        block_end_info: Option<BlockEndInfo>,
        next_epoch_state: Option<EpochState>,
        subscribable_events: Planned<Vec<ContractEvent>>,
    ) -> Self {
        let next_version = first_version + to_commit.len() as Version;
        assert_eq!(next_version, result_state.latest().next_version());
        if is_block {
            // If it's a block, ensure it ends with state checkpoint.
            assert!(to_commit.is_empty() || to_commit.ends_with_sole_checkpoint());
            assert!(result_state.is_checkpoint());
        } else {
            // If it's not, there shouldn't be any transaction to be discarded or retried.
            assert!(to_discard.is_empty() && to_retry.is_empty());
        }

        Self::new_impl(Inner {
            is_block,
            first_version,
            statuses_for_input_txns,
            to_commit,
            to_discard,
            to_retry,
            result_state,
            state_reads,
            block_end_info,
            next_epoch_state,
            subscribable_events,
        })
    }

    pub fn new_empty(state: LedgerState) -> Self {
        Self::new_impl(Inner {
            is_block: false,
            first_version: state.next_version(),
            statuses_for_input_txns: vec![],
            to_commit: TransactionsToKeep::new_empty(),
            to_discard: TransactionsWithOutput::new_empty(),
            to_retry: TransactionsWithOutput::new_empty(),
            state_reads: ShardedStateCache::new_empty(state.version()),
            result_state: state,
            block_end_info: None,
            next_epoch_state: None,
            subscribable_events: Planned::ready(vec![]),
        })
    }

    pub fn new_dummy_with_input_txns(txns: Vec<Transaction>) -> Self {
        let num_txns = txns.len();
        let success_status = TransactionStatus::Keep(ExecutionStatus::Success);
        Self::new_impl(Inner {
            is_block: false,
            first_version: 0,
            statuses_for_input_txns: vec![success_status; num_txns],
            to_commit: TransactionsToKeep::new_dummy_success(txns),
            to_discard: TransactionsWithOutput::new_empty(),
            to_retry: TransactionsWithOutput::new_empty(),
            result_state: LedgerState::new_empty(),
            state_reads: ShardedStateCache::new_empty(None),
            block_end_info: None,
            next_epoch_state: None,
            subscribable_events: Planned::ready(vec![]),
        })
    }

    pub fn new_dummy() -> Self {
        Self::new_dummy_with_input_txns(vec![])
    }

    pub fn reconfig_suffix(&self) -> Self {
        Self::new_impl(Inner {
            is_block: false,
            first_version: self.next_version(),
            statuses_for_input_txns: vec![],
            to_commit: TransactionsToKeep::new_empty(),
            to_discard: TransactionsWithOutput::new_empty(),
            to_retry: TransactionsWithOutput::new_empty(),
            result_state: self.result_state.clone(),
            state_reads: ShardedStateCache::new_empty(self.next_version().checked_sub(1)),
            block_end_info: None,
            next_epoch_state: self.next_epoch_state.clone(),
            subscribable_events: Planned::ready(vec![]),
        })
    }

    fn new_impl(inner: Inner) -> Self {
        Self {
            inner: Arc::new(DropHelper::new(inner)),
        }
    }

    pub fn num_transactions_to_commit(&self) -> usize {
        self.to_commit.transactions.len()
    }

    pub fn next_version(&self) -> Version {
        self.first_version + self.num_transactions_to_commit() as Version
    }

    pub fn expect_last_version(&self) -> Version {
        self.first_version + self.num_transactions_to_commit() as Version - 1
    }
}

#[derive(Debug)]
pub struct Inner {
    pub is_block: bool,
    pub first_version: Version,
    // Statuses of the input transactions, in the same order as the input transactions.
    // Contains BlockMetadata/Validator transactions,
    // but doesn't contain StateCheckpoint/BlockEpilogue, as those get added during execution
    pub statuses_for_input_txns: Vec<TransactionStatus>,
    // List of all transactions to be committed, including StateCheckpoint/BlockEpilogue if needed.
    pub to_commit: TransactionsToKeep,
    pub to_discard: TransactionsWithOutput,
    pub to_retry: TransactionsWithOutput,

    pub result_state: LedgerState,
    /// State items read during execution, useful for calculating the state storge usage and
    /// indices used by the db pruner.
    pub state_reads: ShardedStateCache,

    /// Optional StateCheckpoint payload
    pub block_end_info: Option<BlockEndInfo>,
    /// Optional EpochState payload.
    /// Only present if the block is the last block of an epoch, and is parsed output of the
    /// state cache.
    pub next_epoch_state: Option<EpochState>,
    pub subscribable_events: Planned<Vec<ContractEvent>>,
}

impl Inner {
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
            &aborts[..(aborts.len().min(3))]
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
