// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::{
    fairness::{
        conflict_key::{
            entry_fun::EntryFunKey, entry_fun_module::EntryFunModuleKey, txn_sender::TxnSenderKey,
            ConflictKeyRegistry,
        },
        conflict_zone::ConflictZone,
        pending_zone::PendingZone,
    },
    TransactionShuffler,
};
use aptos_types::transaction::SignedTransaction;
use itertools::zip_eq;
use selection_tracker::SelectionTracker;
use std::collections::BTreeSet;

pub(crate) mod conflict_key;
mod conflict_zone;
mod pending_zone;
mod selection_tracker;

#[cfg(test)]
mod tests;

type TxnIdx = usize;

#[derive(Debug)]
pub struct FairnessShuffler {
    pub sender_conflict_window_size: usize,
    pub module_conflict_window_size: usize,
    pub entry_fun_conflict_window_size: usize,
}

impl FairnessShuffler {
    fn conflict_key_registries(txns: &[SignedTransaction]) -> [ConflictKeyRegistry; 3] {
        [
            ConflictKeyRegistry::build::<TxnSenderKey, SignedTransaction>(txns),
            ConflictKeyRegistry::build::<EntryFunModuleKey, SignedTransaction>(txns),
            ConflictKeyRegistry::build::<EntryFunKey, SignedTransaction>(txns),
        ]
    }

    fn window_sizes(&self) -> [usize; 3] {
        [
            self.sender_conflict_window_size,
            self.module_conflict_window_size,
            self.entry_fun_conflict_window_size,
        ]
    }
}

impl TransactionShuffler for FairnessShuffler {
    fn shuffle(&self, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        let conflict_key_registries = Self::conflict_key_registries(&txns);
        let order =
            FairnessShufflerImpl::new(&conflict_key_registries, self.window_sizes()).shuffle();
        reorder(txns, &order)
    }
}

fn reorder<T: Clone>(txns: Vec<T>, order: &[TxnIdx]) -> Vec<T> {
    assert_eq!(txns.len(), order.len());
    order.iter().map(|idx| txns[*idx].clone()).collect()
}

struct FairnessShufflerImpl<'a, const NUM_CONFLICT_ZONES: usize> {
    conflict_zones: [ConflictZone<'a>; NUM_CONFLICT_ZONES],
    pending_zones: [PendingZone<'a>; NUM_CONFLICT_ZONES],
    selected_order: Vec<TxnIdx>,
    selection_tracker: SelectionTracker,
}

impl<'a, const NUM_CONFLICT_ZONES: usize> FairnessShufflerImpl<'a, NUM_CONFLICT_ZONES> {
    pub fn new(
        conflict_key_registries: &'a [ConflictKeyRegistry; NUM_CONFLICT_ZONES],
        window_sizes: [usize; NUM_CONFLICT_ZONES],
    ) -> Self {
        let num_txns = conflict_key_registries[0].num_txns();
        assert!(conflict_key_registries
            .iter()
            .skip(1)
            .all(|r| r.num_txns() == num_txns));

        Self {
            selected_order: Vec::with_capacity(num_txns),
            selection_tracker: SelectionTracker::new(num_txns),
            conflict_zones: ConflictZone::build_zones(conflict_key_registries, window_sizes),
            pending_zones: PendingZone::build_zones(conflict_key_registries),
        }
    }

    /// Spread (delay) transactions that have conflicts with adjacent previous transactions
    /// according to multiple dimensions (`ConflictKey`s). Invariant is held that for each conflict
    /// key, i.e. the transaction sender, the module, and the entry function, etc., the order of
    /// transactions with the same key is preserved -- unless the key is exempt from conflict.
    ///
    /// For example, all transactions from a single sender will preserve their order; all transactions
    /// from the same module will preserve their order, unless they are of the aptos framework
    /// module -- p2p transfers of APT can violate this invariant.
    ///
    /// Each transaction comes at most once out of `self.selection_tracker.next_unselected()` for
    /// both passes, that's O(2n). And each transaction comes out of each conflict zones at most
    /// once, that's O(3n). In either case, the transaction is examined by going through all 3
    /// conflict zones and all 3 pending zones. So the time complexity is O(9n) = O(n). Or if we
    /// consider `NUM_CONFLICT_ZONES = m`, the time complexity is O(m*m*n).
    pub fn shuffle(mut self) -> Vec<TxnIdx> {
        // First pass, only select transactions with no conflicts in all conflict zones
        while let Some(txn_idx) = self.selection_tracker.next_unselected() {
            if !self.is_conflict(txn_idx) && !self.is_head_of_line_blocked(txn_idx) {
                self.select_and_select_unconflicted(txn_idx, false /* is_pending */)
            } else {
                self.add_pending(txn_idx);
            }
        }

        // Second pass, select previously pending txns in order,
        //   with newly un-conflicted txns jumping the line
        self.selection_tracker.new_pass();
        while let Some(txn_idx) = self.selection_tracker.next_unselected() {
            self.select_and_select_unconflicted(txn_idx, true /* is_pending */);
        }

        self.selected_order
    }

    fn select_and_select_unconflicted(&mut self, txn_idx: TxnIdx, is_pending: bool) {
        let mut maybe_unconflicted = self.select(txn_idx, is_pending);
        while let Some(txn_idx) = maybe_unconflicted.pop_first() {
            if !self.is_conflict(txn_idx) && !self.is_head_of_line_blocked(txn_idx) {
                maybe_unconflicted.extend(self.select(txn_idx, true /* is_pending */))
            }
        }
    }

    /// Select a transaction and return potentially un-conflicted transactions
    fn select(&mut self, txn_idx: TxnIdx, is_pending: bool) -> BTreeSet<TxnIdx> {
        self.selection_tracker.mark_selected(txn_idx);
        self.selected_order.push(txn_idx);
        if is_pending {
            self.pop_pending(txn_idx);
        }

        let mut maybe_unconflicted = BTreeSet::new();
        for (conflict_zone, pending_zone) in
            zip_eq(&mut self.conflict_zones, &mut self.pending_zones)
        {
            if let Some(key_id) = conflict_zone.add(txn_idx) {
                if let Some(pending) = pending_zone.first_pending_on_key(key_id) {
                    maybe_unconflicted.insert(pending);
                }
            }
        }

        maybe_unconflicted
    }

    fn is_conflict(&self, txn_idx: TxnIdx) -> bool {
        self.conflict_zones.iter().any(|z| z.is_conflict(txn_idx))
    }

    fn is_head_of_line_blocked(&self, txn_idx: TxnIdx) -> bool {
        self.pending_zones
            .iter()
            .any(|z| z.head_of_line_blocked(txn_idx))
    }

    fn add_pending(&mut self, txn_idx: TxnIdx) {
        self.pending_zones.iter_mut().for_each(|z| z.add(txn_idx));
    }

    fn pop_pending(&mut self, txn_idx: TxnIdx) {
        self.pending_zones.iter_mut().for_each(|z| z.pop(txn_idx));
    }
}

#[cfg(test)]
mod test_utils {
    use crate::transaction_shuffler::fairness::FairnessShuffler;
    use proptest::prelude::*;

    impl FairnessShuffler {
        pub fn new_for_test(
            sender_conflict_window_size: usize,
            module_conflict_window_size: usize,
            entry_fun_conflict_window_size: usize,
        ) -> Self {
            Self {
                sender_conflict_window_size,
                module_conflict_window_size,
                entry_fun_conflict_window_size,
            }
        }
    }

    impl Arbitrary for FairnessShuffler {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            (0..10usize, 0..10usize, 0..10usize)
                .prop_map(
                    |(
                        sender_conflict_window_size,
                        module_conflict_window_size,
                        entry_fun_conflict_window_size,
                    )| {
                        FairnessShuffler {
                            sender_conflict_window_size,
                            module_conflict_window_size,
                            entry_fun_conflict_window_size,
                        }
                    },
                )
                .boxed()
        }
    }
}
