// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::{ensure, Result};
use aptos_crypto::HashValue;
use aptos_drop_helper::DropHelper;
use aptos_storage_interface::cached_state_view::ShardedStateCache;
use aptos_types::{
    contract_event::ContractEvent,
    proof::accumulator::InMemoryTransactionAccumulator,
    state_store::ShardedStateUpdates,
    transaction::{
        block_epilogue::BlockEndInfo, Transaction, TransactionInfo, TransactionOutput,
        TransactionStatus, Version,
    },
};
use derive_more::Deref;
use itertools::zip_eq;
use std::sync::Arc;

#[derive(Clone, Debug, Default, Deref)]
pub struct LedgerUpdateOutput {
    #[deref]
    inner: Arc<DropHelper<Inner>>,
}

impl LedgerUpdateOutput {
    pub fn new_empty(transaction_accumulator: Arc<InMemoryTransactionAccumulator>) -> Self {
        Self::new_impl(Inner::new_empty(transaction_accumulator))
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_dummy_with_input_txns(txns: Vec<Transaction>) -> Self {
        Self::new_impl(Inner::new_dummy_with_input_txns(txns))
    }

    pub fn new_dummy_with_root_hash(root_hash: HashValue) -> Self {
        Self::new_impl(Inner::new_dummy_with_root_hash(root_hash))
    }

    pub fn reconfig_suffix(&self) -> Self {
        Self::new_impl(Inner::new_empty(self.transaction_accumulator.clone()))
    }

    pub fn new(
        statuses_for_input_txns: Vec<TransactionStatus>,
        transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        transaction_infos: Vec<TransactionInfo>,
        per_version_state_updates: Vec<ShardedStateUpdates>,
        subscribable_events: Vec<ContractEvent>,
        transaction_info_hashes: Vec<HashValue>,
        state_updates_until_last_checkpoint: Option<ShardedStateUpdates>,
        sharded_state_cache: ShardedStateCache,
        transaction_accumulator: Arc<InMemoryTransactionAccumulator>,
        parent_accumulator: Arc<InMemoryTransactionAccumulator>,
        block_end_info: Option<BlockEndInfo>,
    ) -> Self {
        Self::new_impl(Inner {
            statuses_for_input_txns,
            transactions,
            transaction_outputs,
            transaction_infos,
            per_version_state_updates,
            subscribable_events,
            transaction_info_hashes,
            state_updates_until_last_checkpoint,
            sharded_state_cache,
            transaction_accumulator,
            parent_accumulator,
            block_end_info,
        })
    }

    fn new_impl(inner: Inner) -> Self {
        Self {
            inner: Arc::new(DropHelper::new(inner)),
        }
    }
}

#[derive(Default, Debug)]
pub struct Inner {
    pub statuses_for_input_txns: Vec<TransactionStatus>,
    pub transactions: Vec<Transaction>,
    pub transaction_outputs: Vec<TransactionOutput>,
    pub transaction_infos: Vec<TransactionInfo>,
    pub per_version_state_updates: Vec<ShardedStateUpdates>,
    pub subscribable_events: Vec<ContractEvent>,
    pub transaction_info_hashes: Vec<HashValue>,
    pub state_updates_until_last_checkpoint: Option<ShardedStateUpdates>,
    pub sharded_state_cache: ShardedStateCache,
    /// The in-memory Merkle Accumulator representing a blockchain state consistent with the
    /// `state_tree`.
    pub transaction_accumulator: Arc<InMemoryTransactionAccumulator>,
    pub parent_accumulator: Arc<InMemoryTransactionAccumulator>,
    pub block_end_info: Option<BlockEndInfo>,
}

impl Inner {
    pub fn new_empty(transaction_accumulator: Arc<InMemoryTransactionAccumulator>) -> Self {
        Self {
            parent_accumulator: transaction_accumulator.clone(),
            transaction_accumulator,
            ..Default::default()
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_dummy_with_input_txns(transactions: Vec<Transaction>) -> Self {
        let num_txns = transactions.len();
        Self {
            transactions,
            statuses_for_input_txns: vec![
                TransactionStatus::Keep(
                    aptos_types::transaction::ExecutionStatus::Success
                );
                num_txns
            ],
            ..Default::default()
        }
    }

    pub fn new_dummy_with_root_hash(root_hash: HashValue) -> Self {
        let transaction_accumulator = Arc::new(
            InMemoryTransactionAccumulator::new_empty_with_root_hash(root_hash),
        );
        Self {
            parent_accumulator: transaction_accumulator.clone(),
            transaction_accumulator,
            ..Default::default()
        }
    }

    pub fn txn_accumulator(&self) -> &Arc<InMemoryTransactionAccumulator> {
        &self.transaction_accumulator
    }

    /// Ensure that every block committed by consensus ends with a state checkpoint. That can be
    /// one of the two cases: 1. a reconfiguration (txns in the proposed block after the txn caused
    /// the reconfiguration will be retried) 2. a Transaction::StateCheckpoint at the end of the
    /// block.
    pub fn ensure_ends_with_state_checkpoint(&self) -> Result<()> {
        ensure!(
            self.transactions
                .last()
                .map_or(true, |t| t.is_non_reconfig_block_ending()),
            "Block not ending with a state checkpoint.",
        );
        Ok(())
    }

    pub fn ensure_transaction_infos_match(
        &self,
        transaction_infos: &[TransactionInfo],
    ) -> Result<()> {
        ensure!(
            self.transaction_infos.len() == transaction_infos.len(),
            "Lengths don't match. {} vs {}",
            self.transaction_infos.len(),
            transaction_infos.len(),
        );

        let mut version = self.first_version();
        for (txn_info, expected_txn_info) in
            zip_eq(self.transaction_infos.iter(), transaction_infos.iter())
        {
            ensure!(
                txn_info == expected_txn_info,
                "Transaction infos don't match. version:{version}, txn_info:{txn_info}, expected_txn_info:{expected_txn_info}",
            );
            version += 1;
        }
        Ok(())
    }

    pub fn next_version(&self) -> Version {
        self.transaction_accumulator.num_leaves() as Version
    }

    pub fn first_version(&self) -> Version {
        self.parent_accumulator.num_leaves
    }

    pub fn num_txns(&self) -> usize {
        self.transactions.len()
    }
}
