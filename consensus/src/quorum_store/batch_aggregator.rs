// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::types::{BatchId, Data};
use aptos_crypto::{hash::DefaultHasher, HashValue};
use bcs::to_bytes;

struct IncrementalBatchState {
    txn_fragments: Vec<Data>,
    hasher: DefaultHasher,
    num_bytes: usize,
    max_bytes: usize,
}

impl IncrementalBatchState {
    pub fn from_initial_transactions(transactions: Data, max_bytes: usize) -> Self {
        let mut ret = Self {
            txn_fragments: Vec::new(),
            hasher: DefaultHasher::new(b"QuorumStoreBatch"),
            num_bytes: 0,
            max_bytes,
        };

        ret.append_transactions(transactions);
        ret
    }

    fn num_bytes(&self) -> usize {
        self.num_bytes
    }

    pub fn append_transactions(&mut self, transactions: Data) {
        // optimization to save computation when we overflow max size
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
    }

    pub fn num_fragments(&self) -> usize {
        self.txn_fragments.len()
    }

    pub fn finalize_batch(self) -> (usize, Data, HashValue) {
        (
            self.num_bytes(),
            self.txn_fragments.into_iter().flatten().collect(),
            self.hasher.finish(),
        )
    }
}

/// Aggregates batches and computes digest for a given validator.
pub struct BatchAggregator {
    batch_id: BatchId,
    batch_state: Option<IncrementalBatchState>,
    max_batch_bytes: usize,
}

/// Enum that determines how BatchAggregator handles missing fragments in the stream.
/// For streams arriving on the network, it makes sense to be best effort, while for
/// own stream, it can be asserted that the fragments are in expected order.

pub enum AggregationMode {
    AssertMissedFragment,
    IgnoreMissedFragment,
}

impl BatchAggregator {
    pub fn new(max_batch_size: usize) -> Self {
        Self {
            batch_id: 0,
            batch_state: None,
            max_batch_bytes: max_batch_size,
        }
    }

    fn next_fragment_id(&self) -> usize {
        match &self.batch_state {
            Some(state) => state.num_fragments(),
            None => 0,
        }
    }

    fn missed_fragment(&self, batch_id: BatchId, fragment_id: usize) -> bool {
        batch_id > self.batch_id
            || (batch_id == self.batch_id && fragment_id > self.next_fragment_id())
    }

    /// Appends transactions from a batch fragment, ensuring that the fragment is
    /// consistent with the state being aggregated, and handling missing fragments
    /// according to the provided AggregationMode. Returns whether the fragment was
    /// successfully appended (stale fragments are ignored).
    pub fn append_transactions(
        &mut self,
        batch_id: BatchId,
        fragment_id: usize,
        transactions: Data,
        mode: AggregationMode, //TODO: why is this not part of the struct?
    ) -> bool {
        let missed_fragment = self.missed_fragment(batch_id, fragment_id);
        match mode {
            AggregationMode::AssertMissedFragment => assert!(!missed_fragment),
            AggregationMode::IgnoreMissedFragment => {
                if missed_fragment {
                    // If we started receiving a new batch, allow aggregating it by
                    // clearing the state. Otherwise, when some fragment is skipped
                    // within a batch, this just stops aggregating it.
                    self.batch_state = None;
                }
            }
        }

        match &mut self.batch_state {
            Some(state) => {
                if fragment_id == state.num_fragments() && batch_id == self.batch_id {
                    state.append_transactions(transactions);
                    state.num_bytes() <= self.max_batch_bytes
                } else {
                    false
                }
            }
            None => {
                if fragment_id == 0 && batch_id > self.batch_id {
                    self.batch_id = batch_id;
                    self.batch_state = Some(IncrementalBatchState::from_initial_transactions(
                        transactions,
                        self.max_batch_bytes,
                    ));
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn end_batch(
        &mut self,
        batch_id: BatchId,
        fragment_id: usize,
        transactions: Data,
        mode: AggregationMode,
    ) -> Option<(usize, Data, HashValue)> {
        if self.append_transactions(batch_id, fragment_id, transactions, mode) {
            Some(self.batch_state.take().unwrap().finalize_batch())
        } else {
            None
        }
    }
}
