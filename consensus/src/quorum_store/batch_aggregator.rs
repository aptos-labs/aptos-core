// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::{
    counters,
    types::{BatchId, SerializedTransaction},
};
use aptos_crypto::{hash::DefaultHasher, HashValue};
use aptos_logger::debug;
use aptos_types::transaction::SignedTransaction;
use bcs::from_bytes;
use std::result::Result;

#[derive(Clone, Debug, PartialEq)]
pub enum BatchAggregatorError {
    DeserializationError,
    SizeLimitExceeded,
    OutdatedFragment,
    MissedFragment,
}

pub(crate) struct IncrementalBatchState {
    num_fragments: usize,
    status: Result<(), BatchAggregatorError>,
    txns: Vec<SignedTransaction>,
    hasher: DefaultHasher,
    num_bytes: usize,
    max_bytes: usize,
}

impl IncrementalBatchState {
    pub(crate) fn new(max_bytes: usize) -> Self {
        Self {
            num_fragments: 0,
            status: Ok(()),
            txns: Vec::new(),
            hasher: DefaultHasher::new(b"QuorumStoreBatch"),
            num_bytes: 0,
            max_bytes,
        }
    }

    pub(crate) fn append_transactions(
        &mut self,
        serialized_txns: Vec<SerializedTransaction>,
    ) -> Result<(), BatchAggregatorError> {
        self.num_fragments = self.num_fragments + 1;

        if self.status.is_ok() {
            // Avoid useless work if batch already has an error.
            for mut txn in serialized_txns {
                self.num_bytes = self.num_bytes + txn.len();
                // TODO: check that it's fine to hash individually (perf and incrementality).
                self.hasher.update(&txn.bytes());

                if self.num_bytes > self.max_bytes {
                    self.status = Err(BatchAggregatorError::SizeLimitExceeded);
                    break;
                }

                match from_bytes(&txn.take_bytes()) {
                    Ok(signed_txn) => self.txns.push(signed_txn),
                    Err(_) => {
                        self.status = Err(BatchAggregatorError::DeserializationError);
                        break;
                    }
                }
            }
        }

        self.status.clone()
    }

    pub(crate) fn num_fragments(&self) -> usize {
        self.num_fragments
    }

    pub(crate) fn finalize_batch(
        self,
    ) -> Result<(usize, Vec<SignedTransaction>, HashValue), BatchAggregatorError> {
        self.status
            .clone()
            .map(|_| (self.num_bytes, self.txns, self.hasher.finish()))
    }
}

/// Aggregates batches and computes digest for a given validator.
pub(crate) struct BatchAggregator {
    batch_id: Option<BatchId>,
    batch_state: Option<IncrementalBatchState>,
    max_batch_bytes: usize,
}

impl BatchAggregator {
    pub(crate) fn new(max_batch_bytes: usize) -> Self {
        Self {
            batch_id: None,
            batch_state: None,
            max_batch_bytes,
        }
    }

    fn next_fragment_id(&self) -> usize {
        match &self.batch_state {
            Some(state) => state.num_fragments(),
            None => 0,
        }
    }

    fn outdated_fragment(&self, batch_id: BatchId, fragment_id: usize) -> bool {
        if let Some(self_batch_id) = self.batch_id {
            let next_fragment_id = self.next_fragment_id();
            if next_fragment_id == 0 {
                // We always append transactions when we create a new state.
                debug_assert!(
                    self.batch_state.is_none(),
                    "No fragments in batch aggregator state"
                );

                // In this case, the next fragment must start self_batch_id + 1
                batch_id <= self_batch_id
            } else {
                (batch_id, fragment_id) < (self_batch_id, self.next_fragment_id())
            }
        } else {
            false
        }
    }

    // Should only be called with a non-outdated fragment.
    fn missed_fragment(&self, batch_id: BatchId, fragment_id: usize) -> bool {
        match self.batch_id {
            Some(self_batch_id) => {
                if batch_id > self_batch_id {
                    self.batch_state.is_some() || fragment_id > 0
                } else {
                    assert!(
                        batch_id == self_batch_id,
                        "Missed fragment called with an outdated fragment"
                    );
                    fragment_id > self.next_fragment_id()
                }
            }
            // Allow larger batch_id (> 0) the first time as quorum store might
            // be recovering from a crash and continuing with a larger batch_id.
            None => fragment_id > 0,
        }
    }

    /// Appends transactions from a batch fragment, ensuring that the fragment is
    /// consistent with the state being aggregated, and the maximum batch size is
    /// being respected. Otherwise, a corresponding error is returned which should
    /// be handled on the caller side (i.e. panic for self, log & ignore for peers).
    pub(crate) fn append_transactions(
        &mut self,
        batch_id: BatchId,
        fragment_id: usize,
        transactions: Vec<SerializedTransaction>,
    ) -> Result<(), BatchAggregatorError> {
        if self.outdated_fragment(batch_id, fragment_id) {
            counters::EXPIRED_BATCH_FRAGMENTS_COUNT.inc();
            // Replay or batch / fragment received out of order.
            return Err(BatchAggregatorError::OutdatedFragment);
        }

        let missed_fragment = self.missed_fragment(batch_id, fragment_id);
        if missed_fragment {
            counters::MISSED_BATCH_FRAGMENTS_COUNT.inc();
            debug!(
                "QS: missed_fragment batch_id: {:?} fragment_id {:?}",
                batch_id, fragment_id
            );
            // If we started receiving a new batch, allow aggregating it by
            // clearing the state. Otherwise, when some fragment is skipped
            // within a batch, this just stops aggregating it.
            self.batch_state = None;
        }

        // Fragment wasn't outdated, fast forward batch id.
        self.batch_id = Some(batch_id);
        if fragment_id == 0 {
            // If a fragment was missed, state should be cleared
            // above, and otherwise, it must be cleared by end_batch.
            debug_assert!(self.batch_state.is_none(), "Batch state not cleared");
            self.batch_state = Some(IncrementalBatchState::new(self.max_batch_bytes));
        }

        if self.batch_state.is_some() {
            self.batch_state
                .as_mut()
                .unwrap()
                .append_transactions(transactions)?
        }
        if missed_fragment {
            Err(BatchAggregatorError::MissedFragment)
        } else {
            Ok(())
        }
    }

    pub(crate) fn end_batch(
        &mut self,
        batch_id: BatchId,
        fragment_id: usize,
        transactions: Vec<SerializedTransaction>,
    ) -> Result<(usize, Vec<SignedTransaction>, HashValue), BatchAggregatorError> {
        let append_res = self.append_transactions(batch_id, fragment_id, transactions);
        if append_res.is_ok()
            || (append_res == Err(BatchAggregatorError::MissedFragment)
                && self.batch_state.is_some())
        {
            self.batch_state
                .take()
                .expect("Batch state must exist")
                .finalize_batch()
        } else {
            Err(append_res.unwrap_err())
        }
    }
}
