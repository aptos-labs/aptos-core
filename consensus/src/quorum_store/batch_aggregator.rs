// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]
#![allow(unused_variables)]

use crate::quorum_store::{
    counters,
    types::{BatchId, SerializedTransaction},
};
use aptos_crypto::{hash::DefaultHasher, HashValue};
use aptos_logger::{error, warn};
use aptos_types::transaction::SignedTransaction;
use bcs::from_bytes;
use std::result::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
/// Errors that are returned when aggregation fails. Note that aggregation may
/// succeed despite e.g. a missed fragment, if the received fragment starts a new,
/// higher batch. In this case the error is not returned (note that we do have
/// a separate counter that is increased regardless of whether aggregation succeeded).
pub enum BatchAggregationError {
    DeserializationError,
    SizeLimitExceeded,
    OutdatedFragment,
    MissedFragment,
}

pub(crate) struct IncrementalBatchState {
    num_fragments: usize,
    status: Result<(), BatchAggregationError>,
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
    ) -> Result<(), BatchAggregationError> {
        self.num_fragments += 1;

        if self.status.is_ok() {
            // Avoid useless work if batch already has an error.
            for mut txn in serialized_txns {
                self.num_bytes += txn.len();
                // TODO: check that it's fine to hash individually (perf and incrementality).
                // TODO: can we re-use the hash (when serving txns later).
                self.hasher.update(txn.bytes());

                if self.num_bytes > self.max_bytes {
                    self.status = Err(BatchAggregationError::SizeLimitExceeded);
                    break;
                }

                match from_bytes(&txn.take_bytes()) {
                    Ok(signed_txn) => self.txns.push(signed_txn),
                    Err(_) => {
                        self.status = Err(BatchAggregationError::DeserializationError);
                        break;
                    },
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
    ) -> Result<(usize, Vec<SignedTransaction>, HashValue), BatchAggregationError> {
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

    #[inline]
    fn is_new_batch(batch_id: BatchId, prev_batch_id: BatchId) -> bool {
        // If the nonce has changed, this is a new batch (after validator DB was wiped).
        batch_id.nonce != prev_batch_id.nonce || batch_id > prev_batch_id
    }

    fn is_outdated_fragment(&self, batch_id: BatchId, fragment_id: usize) -> bool {
        if let Some(self_batch_id) = self.batch_id {
            if Self::is_new_batch(batch_id, self_batch_id) {
                return false;
            }
            let next_fragment_id = self.next_fragment_id();
            if next_fragment_id == 0 {
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
    fn is_missed_fragment(&self, batch_id: BatchId, fragment_id: usize) -> bool {
        match self.batch_id {
            Some(self_batch_id) => {
                if Self::is_new_batch(batch_id, self_batch_id) {
                    self.batch_state.is_some() || fragment_id > 0
                } else {
                    assert!(
                        batch_id == self_batch_id,
                        "Missed fragment called with an outdated fragment"
                    );
                    fragment_id > self.next_fragment_id()
                }
            },
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
    ) -> Result<(), BatchAggregationError> {
        if self.is_outdated_fragment(batch_id, fragment_id) {
            counters::EXPIRED_BATCH_FRAGMENTS_COUNT.inc();
            // Replay or batch / fragment received out of order.
            return Err(BatchAggregationError::OutdatedFragment);
        }
        let missed_fragment = self.is_missed_fragment(batch_id, fragment_id);

        // Fragment wasn't outdated, fast forward batch id.
        self.batch_id = Some(batch_id);

        if missed_fragment {
            counters::MISSED_BATCH_FRAGMENTS_COUNT.inc();
            warn!(
                "QS: missed_fragment batch_id: {:?} fragment_id {:?}",
                batch_id, fragment_id
            );

            // If we started receiving a new batch (fragment_id = 0), we allow aggregating
            // it by clearing the state. But by setting state to None in all cases, we are
            // indicating that the batch was invalid & we expect to aggregate higher batches.
            // Hence all future fragments from the invalid batch will be 'Outdated'.
            self.batch_state = None;

            if fragment_id != 0 {
                // fragment was skipped and we can't aggregate a new batch. We skip the
                // message (as an optimization) and return the corresponding error.
                return Err(BatchAggregationError::MissedFragment);
            }
        }

        if fragment_id == 0 {
            if self.batch_state.is_some() {
                // If a fragment was missed, state should be cleared
                // above, and otherwise, it must be cleared by end_batch.
                error!("Batch state not cleared for a new batch");
            }
            self.batch_state = Some(IncrementalBatchState::new(self.max_batch_bytes));
        }

        if self.batch_state.is_some() {
            self.batch_state
                .as_mut()
                .unwrap()
                .append_transactions(transactions)?
        }
        Ok(())
    }

    pub(crate) fn end_batch(
        &mut self,
        batch_id: BatchId,
        fragment_id: usize,
        transactions: Vec<SerializedTransaction>,
    ) -> Result<(usize, Vec<SignedTransaction>, HashValue), BatchAggregationError> {
        match self.append_transactions(batch_id, fragment_id, transactions) {
            Ok(()) => self
                .batch_state
                .take()
                .expect("Batch state must exist")
                .finalize_batch(),
            Err(e) => Err(e),
        }
    }
}
