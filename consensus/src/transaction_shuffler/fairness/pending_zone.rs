// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::fairness::{
    conflict_key::{ConflictKeyId, ConflictKeyRegistry, MapByKeyId},
    TxnIdx,
};
use std::collections::VecDeque;

/// A queue for each confclit Key, represented by `ConflictKeyId`s managed by `ConflictKeyRegistry`.
#[derive(Debug)]
pub(crate) struct PendingZone<'a> {
    key_registry: &'a ConflictKeyRegistry,
    pending_by_key: MapByKeyId<VecDeque<TxnIdx>>,
}

impl<'a> PendingZone<'a> {
    pub fn build_zones<const NUM_CONFLICT_ZONES: usize>(
        key_registries: &'a [ConflictKeyRegistry; NUM_CONFLICT_ZONES],
    ) -> [Self; NUM_CONFLICT_ZONES] {
        key_registries
            .iter()
            .map(Self::new)
            .collect::<Vec<_>>()
            .try_into()
            .expect("key_registries and the return type must have the same length.")
    }

    fn new(key_registry: &'a ConflictKeyRegistry) -> Self {
        Self {
            key_registry,
            pending_by_key: key_registry.new_map_by_id(),
        }
    }

    pub fn add(&mut self, txn_idx: TxnIdx) {
        let key_id = self.key_registry.key_id_for_txn(txn_idx);
        if !self.key_registry.is_conflict_exempt(key_id) {
            self.pending_by_key.get_mut(key_id).push_back(txn_idx);
        }
    }

    pub fn pop(&mut self, txn_idx: TxnIdx) {
        let key_id = self.key_registry.key_id_for_txn(txn_idx);
        if !self.key_registry.is_conflict_exempt(key_id) {
            let popped = self
                .pending_by_key
                .get_mut(key_id)
                .pop_front()
                .expect("Must exist");
            assert_eq!(popped, txn_idx);
        }
    }

    pub fn head_of_line_blocked(&self, txn_idx: TxnIdx) -> bool {
        let key_id = self.key_registry.key_id_for_txn(txn_idx);
        if self.key_registry.is_conflict_exempt(key_id) {
            false
        } else {
            match self.pending_by_key.get(key_id).front() {
                Some(front) => *front < txn_idx,
                None => false,
            }
        }
    }

    pub fn first_pending_on_key(&self, key_id: ConflictKeyId) -> Option<TxnIdx> {
        self.pending_by_key.get(key_id).front().cloned()
    }
}
