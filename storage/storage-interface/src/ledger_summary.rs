// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    state_store::{
        state_delta::StateDelta,
        state_view::{async_proof_fetcher::AsyncProofFetcher, cached_state_view::CachedStateView},
    },
    DbReader,
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_types::{
    proof::accumulator::{InMemoryAccumulator, InMemoryTransactionAccumulator},
    state_store::{state_storage_usage::StateStorageUsage, StateViewId},
    transaction::Version,
};
use std::sync::Arc;

/// A wrapper of the in-memory state sparse merkle tree and the transaction accumulator that
/// represent a specific state collectively. Usually it is a state after executing a block.
#[derive(Clone, Debug)]
pub struct LedgerSummary {
    /// The in-memory representation of state after execution.
    pub state: Arc<StateDelta>,

    /// The in-memory Merkle Accumulator representing a blockchain state consistent with the
    /// `state_tree`.
    pub transaction_accumulator: Arc<InMemoryTransactionAccumulator>,
}

impl LedgerSummary {
    pub fn state(&self) -> &Arc<StateDelta> {
        &self.state
    }

    pub fn txn_accumulator(&self) -> &Arc<InMemoryTransactionAccumulator> {
        &self.transaction_accumulator
    }

    pub fn version(&self) -> Option<Version> {
        self.num_transactions().checked_sub(1)
    }

    pub fn num_transactions(&self) -> u64 {
        self.txn_accumulator().num_leaves()
    }

    pub fn state_id(&self) -> HashValue {
        self.txn_accumulator().root_hash()
    }

    pub fn new(
        state: Arc<StateDelta>,
        transaction_accumulator: Arc<InMemoryTransactionAccumulator>,
    ) -> Self {
        assert_eq!(
            state.current_version.map_or(0, |v| v + 1),
            transaction_accumulator.num_leaves()
        );
        Self {
            state,
            transaction_accumulator,
        }
    }

    pub fn new_at_state_checkpoint(
        state_root_hash: HashValue,
        state_usage: StateStorageUsage,
        frozen_subtrees_in_accumulator: Vec<HashValue>,
        num_leaves_in_accumulator: u64,
    ) -> Self {
        let state = Arc::new(StateDelta::new_at_checkpoint(
            state_root_hash,
            state_usage,
            num_leaves_in_accumulator.checked_sub(1),
        ));
        let transaction_accumulator = Arc::new(
            InMemoryAccumulator::new(frozen_subtrees_in_accumulator, num_leaves_in_accumulator)
                .expect("The startup info read from storage should be valid."),
        );

        Self::new(state, transaction_accumulator)
    }

    pub fn new_empty() -> Self {
        Self::new(
            Arc::new(StateDelta::new_empty()),
            Arc::new(InMemoryAccumulator::new_empty()),
        )
    }

    pub fn is_same_view(&self, rhs: &Self) -> bool {
        self.state().has_same_current_state(rhs.state())
            && self.transaction_accumulator.root_hash() == rhs.transaction_accumulator.root_hash()
    }

    pub fn verified_state_view(
        &self,
        id: StateViewId,
        reader: Arc<dyn DbReader>,
        proof_fetcher: Arc<AsyncProofFetcher>,
    ) -> Result<CachedStateView> {
        Ok(CachedStateView::new(
            id,
            reader,
            self.transaction_accumulator.num_leaves(),
            self.state.current.clone(),
            proof_fetcher,
        )?)
    }
}

impl Default for LedgerSummary {
    fn default() -> Self {
        Self::new_empty()
    }
}
