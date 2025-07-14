// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_view::hot_state_view::HotStateView;
use aptos_experimental_layered_map::LayeredMap;
use aptos_types::state_store::{
    hot_state::THotStateSlot, state_key::StateKey, state_slot::StateSlot,
};
use std::{collections::HashMap, sync::Arc};

pub(crate) struct HotStateLRU<'a> {
    /// The entire hot state resulted from committed transactions.
    committed: Arc<dyn HotStateView>,
    /// Additional entries resulted from previous speculative execution.
    overlay: &'a LayeredMap<StateKey, StateSlot>,
    /// The new entries from the current execution.
    pub pending: HashMap<StateKey, StateSlot>,
    /// Points to the latest entry. `None` if empty.
    pub head: Option<StateKey>,
    /// Points to the oldest entry. `None` if empty.
    pub tail: Option<StateKey>,
    pub num_entries_changed: isize,
}

impl<'a> HotStateLRU<'a> {
    pub fn new(
        committed: Arc<dyn HotStateView>,
        overlay: &'a LayeredMap<StateKey, StateSlot>,
        head: Option<StateKey>,
        tail: Option<StateKey>,
    ) -> Self {
        Self {
            committed,
            overlay,
            pending: HashMap::new(),
            head,
            tail,
            num_entries_changed: 0,
        }
    }

    pub fn insert(&mut self, key: StateKey, slot: StateSlot) {
        assert!(slot.is_hot());
        self.delete(&key);
        self.insert_as_head(key, slot);
    }

    fn insert_as_head(&mut self, key: StateKey, mut slot: StateSlot) {
        self.num_entries_changed += 1;
        match self.head.take() {
            Some(head) => {
                let mut old_head_slot = self.expect_hot_slot(&head);
                old_head_slot.set_prev(Some(key.clone()));
                slot.set_prev(None);
                slot.set_next(Some(head.clone()));
                self.pending.insert(head, old_head_slot);
                self.pending.insert(key.clone(), slot);
                self.head = Some(key);
            },
            None => {
                slot.set_prev(None);
                slot.set_next(None);
                self.pending.insert(key.clone(), slot);
                self.head = Some(key.clone());
                self.tail = Some(key);
            },
        }
    }

    pub fn delete(&mut self, key: &StateKey) {
        let old_entry = match self.get_slot(key) {
            Some(e) => e,
            None => return,
        };
        assert!(old_entry.is_hot());
        self.num_entries_changed -= 1;

        match old_entry.prev() {
            Some(prev_key) => {
                let mut prev_entry = self.expect_hot_slot(prev_key);
                prev_entry.set_next(old_entry.next().cloned());
                self.pending.insert(prev_key.clone(), prev_entry);
            },
            None => {
                // There is no newer entry. The current key was the head.
                self.head = old_entry.next().cloned();
            },
        }

        match old_entry.next() {
            Some(next_key) => {
                let mut next_entry = self.expect_hot_slot(next_key);
                next_entry.set_prev(old_entry.prev().cloned());
                self.pending.insert(next_key.clone(), next_entry);
            },
            None => {
                // There is no older entry. The current key was the tail.
                self.tail = old_entry.prev().cloned();
            },
        }

        self.pending.insert(key.clone(), old_entry.to_cold());
    }

    fn get_slot(&self, key: &StateKey) -> Option<StateSlot> {
        if let Some(entry) = self.pending.get(key) {
            return Some(entry.clone());
        }

        if let Some(v) = self.overlay.get(key) {
            return Some(v);
        }

        self.committed.get_state_slot(key)
    }

    fn expect_hot_slot(&self, key: &StateKey) -> StateSlot {
        let slot = self.get_slot(key).expect("Given key is expected to exist.");
        assert!(slot.is_hot());
        slot
    }
}

/*
#[cfg(test)]
mod tests {
    use super::HotStateLRU;
    use crate::state_store::state_view::hot_state_view::HotStateView;
    use aptos_experimental_layered_map::{LayeredMap, MapLayer};
    use aptos_types::{
        state_store::{
            hot_state::LRUEntry, state_key::StateKey, state_slot::StateSlot,
            state_value::StateValue,
        },
        transaction::Version,
    };
    use lru::LruCache;
    use proptest::{
        collection::{hash_map, hash_set, vec},
        prelude::*,
        sample,
        strategy::Strategy,
    };
    use std::{collections::HashMap, sync::Arc};

    #[derive(Debug)]
    struct HotState {
        inner: HashMap<StateKey, StateSlot>,
    }

    impl HotStateView for HotState {
        fn get_state_slot(&self, state_key: &StateKey) -> Option<StateSlot> {
            self.inner.get(state_key).map(|v| v.clone())
        }
    }

    struct LRUTest {
        hot_state: Arc<dyn HotStateView>,
        _base_layer: MapLayer<StateKey, StateSlot>,
        _top_layer: MapLayer<StateKey, StateSlot>,
        overlay: LayeredMap<StateKey, StateSlot>,
    }

    impl LRUTest {
        fn new_empty() -> Self {
            let hot_state = Arc::new(HotState {
                inner: HashMap::new(),
            });
            let base_layer = MapLayer::new_family("test");
            let top_layer = base_layer.clone();
            let overlay = base_layer.view_layers_after(&top_layer);

            Self {
                hot_state,
                _base_layer: base_layer,
                _top_layer: top_layer,
                overlay,
            }
        }
    }

    fn arb_state_slot() -> impl Strategy<Value = StateSlot> {
        (any::<StateValue>(), any::<Version>()).prop_flat_map(|(value, hot_since_version)| {
            prop_oneof![
                3 => Just(StateSlot::HotOccupied { value_version: 0, value: value.clone(), hot_since_version, lru_info: LRUEntry {prev: None, next: None}}),
                2 => Just(StateSlot::HotVacant { hot_since_version, lru_info: LRUEntry {prev: None, next: None}}),
                // 1 => Just(StateSlot::ColdOccupied { value_version: 0, value }),
                // 1 => Just(StateSlot::ColdVacant),
            ]
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1))]

        #[test]
        fn test_empty_overlay(
            kvs in hash_set(any::<StateKey>(), 1..10)
                .prop_flat_map(|keys| {
                    let pool: Vec<_> = keys.into_iter().collect();
                    vec((sample::select(pool), arb_state_slot()), 1..50)
                })
        ) {
            // println!("{kvs:?}");

            let test_obj = LRUTest::new_empty();
            let mut lru = HotStateLRU::new(
                Arc::clone(&test_obj.hot_state),
                &test_obj.overlay,
                None,
                None,
            );
            assert_eq!(lru.head, None);
            assert_eq!(lru.tail, None);
            assert!(lru.pending.is_empty());

            let mut also_lru = LruCache::unbounded();

            for (key, value) in &kvs {
                if value.is_hot() {
                    lru.insert(key.clone(), value.clone());
                    also_lru.put(key, value);
                } else {
                    if also_lru.contains(key) {
                        also_lru.pop(key);
                        lru.delete(key);
                    }
                }
                assert_eq!(lru.head, also_lru.peek_mru().map(|(k, _v)| (*k).clone()));
                assert_eq!(lru.tail, also_lru.peek_lru().map(|(k, _v)| (*k).clone()));
                assert_eq!(lru.collect_all_from_head(), also_lru.iter().map(|(k, v)| ((*k).clone(), (*v).clone())).collect::<Vec<_>>());
            }
        }
    }
}
*/
