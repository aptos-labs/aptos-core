// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::deprecated_fairness::{
    conflict_key::{ConflictKeyId, ConflictKeyRegistry, MapByKeyId},
    TxnIdx,
};
use std::collections::VecDeque;

/// A sliding window of transactions (TxnIds), represented by `ConflictKeyId`s extracted from a
/// specific `ConflictKey`, managed by a specific `ConflictKeyRegistry`.
#[derive(Debug)]
pub(crate) struct ConflictZone<'a> {
    sliding_window_size: usize,
    sliding_window: VecDeque<ConflictKeyId>,
    /// Number of transactions in the sliding window for each key_id. `ConflictZone::is_conflict(key)`
    /// returns true is the count for `key` is greater than 0, unless the key is exempt from conflict.
    counts_by_id: MapByKeyId<usize>,
    key_registry: &'a ConflictKeyRegistry,
}

impl<'a> ConflictZone<'a> {
    pub fn build_zones<const NUM_CONFLICT_ZONES: usize>(
        key_registries: &'a [ConflictKeyRegistry; NUM_CONFLICT_ZONES],
        window_sizes: [usize; NUM_CONFLICT_ZONES],
    ) -> [Self; NUM_CONFLICT_ZONES] {
        itertools::zip_eq(key_registries.iter(), window_sizes)
            .map(|(registry, window_size)| Self::new(registry, window_size))
            .collect::<Vec<_>>()
            .try_into()
            .expect("key_registries and window_sizes must have the same length.")
    }

    fn new(key_registry: &'a ConflictKeyRegistry, sliding_window_size: usize) -> Self {
        Self {
            sliding_window_size,
            sliding_window: VecDeque::with_capacity(sliding_window_size + 1),
            counts_by_id: key_registry.new_map_by_id(),
            key_registry,
        }
    }

    pub fn is_conflict(&self, txn_idx: TxnIdx) -> bool {
        let key_id = self.key_registry.key_id_for_txn(txn_idx);
        if self.key_registry.is_conflict_exempt(key_id) {
            false
        } else {
            *self.counts_by_id.get(key_id) > 0
        }
    }

    /// Append a new transaction to the sliding window and
    /// return the key_id that's no longer in conflict as a result if there is one.
    pub fn add(&mut self, txn_idx: TxnIdx) -> Option<ConflictKeyId> {
        let key_id = self.key_registry.key_id_for_txn(txn_idx);

        *self.counts_by_id.get_mut(key_id) += 1;
        self.sliding_window.push_back(key_id);
        if self.sliding_window.len() > self.sliding_window_size {
            if let Some(removed_key_id) = self.sliding_window.pop_front() {
                let count = self.counts_by_id.get_mut(removed_key_id);
                *count -= 1;
                if *count == 0 && !self.key_registry.is_conflict_exempt(removed_key_id) {
                    return Some(removed_key_id);
                }
            }
        }
        None
    }
}
