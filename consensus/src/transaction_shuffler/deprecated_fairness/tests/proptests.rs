// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::deprecated_fairness::{
    conflict_key::{
        test_utils::{FakeEntryFunKey, FakeEntryFunModuleKey, FakeSenderKey, FakeTxn},
        ConflictKeyRegistry, MapByKeyId,
    },
    reorder, FairnessShuffler, FairnessShufflerImpl, TxnIdx,
};
use proptest::{collection::vec, prelude::*};
use std::collections::BTreeSet;

fn arb_order(num_txns: usize) -> impl Strategy<Value = Vec<TxnIdx>> {
    Just((0..num_txns).collect::<Vec<_>>()).prop_shuffle()
}

#[derive(Debug, Default, Eq, PartialEq)]
enum OrderOrSet {
    #[default]
    Empty,
    Order(Vec<TxnIdx>),
    Set(BTreeSet<TxnIdx>),
}

impl OrderOrSet {
    fn add(&mut self, idx: TxnIdx, is_conflict_exempt: bool) {
        if self.is_empty() {
            *self = if is_conflict_exempt {
                Self::Set(BTreeSet::new())
            } else {
                Self::Order(Vec::new())
            };
        }

        match self {
            Self::Order(order) => order.push(idx),
            Self::Set(set) => {
                set.insert(idx);
            },
            Self::Empty => unreachable!(),
        }
    }

    fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

fn sort_by_key(
    order: impl IntoIterator<Item = TxnIdx>,
    registry: &ConflictKeyRegistry,
) -> MapByKeyId<OrderOrSet> {
    let mut map: MapByKeyId<OrderOrSet> = registry.new_map_by_id();

    for txn_idx in order {
        let key_id = registry.key_id_for_txn(txn_idx);
        let is_exempt = registry.is_conflict_exempt(key_id);

        map.get_mut(key_id).add(txn_idx, is_exempt);
    }

    map
}

fn assert_invariants(txns: &[FakeTxn], order: Vec<TxnIdx>, registry: &ConflictKeyRegistry) {
    let num_txns = txns.len();
    let original_sorted = sort_by_key(0..num_txns, registry);
    let result_sorted = sort_by_key(order, registry);

    assert_eq!(result_sorted, original_sorted);
}

fn registries(txns: &[FakeTxn]) -> [ConflictKeyRegistry; 3] {
    [
        ConflictKeyRegistry::build::<FakeSenderKey, FakeTxn>(txns),
        ConflictKeyRegistry::build::<FakeEntryFunModuleKey, FakeTxn>(txns),
        ConflictKeyRegistry::build::<FakeEntryFunKey, FakeTxn>(txns),
    ]
}

proptest! {
    #[test]
    fn test_reorder( order in (0..1000usize).prop_flat_map(arb_order) ) {
        let num_txns = order.len();
        let txns = (0..num_txns).collect::<Vec<_>>();

        let reordered = reorder(txns, &order);
        prop_assert_eq!(reordered, order);
    }

    #[test]
    fn test_fairness_shuffler(
        txns in vec(any::<FakeTxn>(), 0..1000),
        shuffler in any::<FairnessShuffler>(),
    ) {
        let registries = registries(&txns);
        let order = FairnessShufflerImpl::new(&registries, shuffler.window_sizes()).shuffle();

        for registry in &registries {
            assert_invariants(&txns, order.clone(), registry);
        }
    }
}
