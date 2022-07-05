// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::types::{BatchId, Data};
use aptos_crypto::{hash::DefaultHasher, HashValue};
// use aptos_logger::debug;
use bcs::to_bytes;

struct IncrementalBatchState {
    txn_fragments: Vec<Data>,
    hasher: DefaultHasher,
    num_bytes: usize,
    max_bytes: usize,
}

impl IncrementalBatchState {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            txn_fragments: Vec::new(),
            hasher: DefaultHasher::new(b"QuorumStoreBatch"),
            num_bytes: 0,
            max_bytes,
        }
    }

    pub fn append_transactions(&mut self, transactions: Data) -> bool {
        // Save computation when we overflow max size.
        if self.num_bytes < self.max_bytes {
            let serialized: Vec<u8> = transactions
                .iter()
                .map(|txn| to_bytes(txn).unwrap())
                .flatten()
                .collect();
            self.num_bytes = self.num_bytes + serialized.len();
            self.hasher.update(&serialized);
        }
        self.txn_fragments.push(transactions);
        self.num_bytes <= self.max_bytes
    }

    pub fn num_fragments(&self) -> usize {
        self.txn_fragments.len()
    }

    pub fn finalize_batch(self) -> (usize, Data, HashValue) {
        (
            self.num_bytes,
            self.txn_fragments.into_iter().flatten().collect(),
            self.hasher.finish(),
        )
    }
}

/// Enum that determines how BatchAggregator handles missing fragments in the stream.
/// For streams arriving on the network, it makes sense to be best effort, while for
/// own stream, we assert that the fragments are in expected order.
pub(crate) enum AggregationMode {
    AssertWrongOrder,
    IgnoreWrongOrder,
}

/// Aggregates batches and computes digest for a given validator.
pub(crate) struct BatchAggregator {
    batch_id: Option<BatchId>,
    batch_state: Option<IncrementalBatchState>,
    max_batch_bytes: usize,
    mode: AggregationMode,
}

impl BatchAggregator {
    pub(crate) fn new(max_batch_size: usize, mode: AggregationMode) -> Self {
        Self {
            batch_id: None,
            batch_state: None,
            max_batch_bytes: max_batch_size,
            mode,
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
            (batch_id, fragment_id) < (self_batch_id, self.next_fragment_id())
        } else {
            false
        }
    }

    fn missed_fragment(&self, batch_id: BatchId, fragment_id: usize) -> bool {
        match self.batch_id {
            Some(self_batch_id) => {
                if batch_id > self_batch_id {
                    self.batch_state.is_some() || fragment_id > 0
                } else {
                    fragment_id > self.next_fragment_id()
                }
            }
            // Allow larger batch_id (> 0) the first time as quorum store might
            // be recovering from a crash and continuing with a larger batch_id.
            None => fragment_id > 0,
        }
    }

    /// Appends transactions from a batch fragment, ensuring that the fragment is
    /// consistent with the state being aggregated, and handling missing fragments
    /// according to the provided AggregationMode. Returns whether the fragment was
    /// successfully appended (stale fragments are ignored).
    pub(crate) fn append_transactions(
        &mut self,
        batch_id: BatchId,
        fragment_id: usize,
        transactions: Data,
    ) -> bool {
        let outdated_fragment = self.outdated_fragment(batch_id, fragment_id);
        let missed_fragment = self.missed_fragment(batch_id, fragment_id);

        match self.mode {
            AggregationMode::AssertWrongOrder => {
                assert!(
                    !outdated_fragment && !missed_fragment,
                    "Wrong batch / fragment ID"
                )
            }
            AggregationMode::IgnoreWrongOrder => {
                if outdated_fragment {
                    // Replay or batch / fragment received out of order - ignore.
                    return false;
                }

                if missed_fragment {
                    // If we started receiving a new batch, allow aggregating it by
                    // clearing the state. Otherwise, when some fragment is skipped
                    // within a batch, this just stops aggregating it.
                    self.batch_state = None;
                }
            }
        }

        if fragment_id == 0 {
            debug_assert!(self.batch_state.is_none());
            self.batch_state = Some(IncrementalBatchState::new(self.max_batch_bytes));
            self.batch_id = Some(batch_id);
        }
        if self.batch_state.is_some() {
            self.batch_state
                .as_mut()
                .unwrap()
                .append_transactions(transactions)
        } else {
            false
        }
    }

    pub(crate) fn end_batch(
        &mut self,
        batch_id: BatchId,
        fragment_id: usize,
        transactions: Data,
    ) -> Option<(usize, Data, HashValue)> {
        if self.append_transactions(batch_id, fragment_id, transactions) {
            Some(self.batch_state.take().unwrap().finalize_batch())
        } else {
            None
        }
    }
}
