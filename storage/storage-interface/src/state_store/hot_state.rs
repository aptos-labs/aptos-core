// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_view::hot_state_view::HotStateView;
use aptos_experimental_layered_map::LayeredMap;
use aptos_logger::prelude::*;
use aptos_types::state_store::{
    hot_state::THotStateSlot, state_key::StateKey, state_slot::StateSlot,
};
use std::{collections::HashMap, sync::Arc};

/// NOTE: this always operates on a single shard.
pub(crate) struct HotStateLRU<'a> {
    /// The entire hot state resulted from committed transactions.
    committed: Arc<dyn HotStateView>,
    /// Additional entries resulted from previous speculative execution.
    overlay: &'a LayeredMap<StateKey, StateSlot>,
    /// The new entries from the current execution.
    pending: HashMap<StateKey, StateSlot>,
    /// Points to the latest entry. `None` if empty.
    head: Option<StateKey>,
    /// Points to the oldest entry. `None` if empty.
    tail: Option<StateKey>,
    num_items: usize,
}

impl<'a> HotStateLRU<'a> {
    pub fn new(
        committed: Arc<dyn HotStateView>,
        overlay: &'a LayeredMap<StateKey, StateSlot>,
        head: Option<StateKey>,
        tail: Option<StateKey>,
        num_items: usize,
    ) -> Self {
        Self {
            committed,
            overlay,
            pending: HashMap::new(),
            head,
            tail,
            num_items,
        }
    }

    pub fn insert(&mut self, key: StateKey, slot: StateSlot) {
        assert!(slot.is_hot());
        self.evict(&key);
        self.insert_as_head(key, slot);
    }

    fn insert_as_head(&mut self, key: StateKey, mut slot: StateSlot) {
        info!("self.num_items: {}", self.num_items);
        self.num_items += 1;
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

    pub fn evict(&mut self, key: &StateKey) {
        let old_entry = match self.get_slot(key) {
            Some(e) => e,
            None => return,
        };
        if old_entry.is_cold() {
            return;
        }
        info!("self.num_items: {}", self.num_items);
        self.num_items -= 1;

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

    pub fn into_updates(
        self,
    ) -> (
        HashMap<StateKey, StateSlot>,
        Option<StateKey>,
        Option<StateKey>,
        usize,
    ) {
        (self.pending, self.head, self.tail, self.num_items)
    }

    #[cfg(test)]
    fn validate_lru(&self) {
        self.validate_from_head();
        self.validate_from_tail();
    }

    #[cfg(test)]
    fn validate_from_head(&self) {
        let mut current = self.head.clone();
        let mut num_visited = 0;
        while let Some(key) = current {
            let slot = self.expect_hot_slot(&key);
            num_visited += 1;
            current = slot.next().cloned();
        }
        assert_eq!(num_visited, self.num_items);
    }

    #[cfg(test)]
    fn validate_from_tail(&self) {
        let mut current = self.tail.clone();
        let mut num_visited = 0;
        while let Some(key) = current {
            let slot = self.expect_hot_slot(&key);
            num_visited += 1;
            current = slot.prev().cloned();
        }
        assert_eq!(num_visited, self.num_items);
    }
}

#[cfg(test)]
mod tests {
    use super::HotStateLRU;
    use crate::state_store::state_view::hot_state_view::HotStateView;
    use aptos_experimental_layered_map::{LayeredMap, MapLayer};
    use aptos_types::{
        state_store::{hot_state::LRUEntry, state_key::StateKey, state_slot::StateSlot},
        transaction::Version,
    };
    use proptest::{
        collection::{hash_set, vec},
        prelude::*,
        sample,
        strategy::Strategy,
    };
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    #[derive(Debug)]
    struct HotState {
        inner: HashMap<StateKey, StateSlot>,
        head: Option<StateKey>,
        tail: Option<StateKey>,
    }

    impl HotStateView for Mutex<HotState> {
        fn get_state_slot(&self, state_key: &StateKey) -> Option<StateSlot> {
            self.lock().unwrap().inner.get(state_key).cloned()
        }
    }

    struct LRUTest {
        hot_state: Arc<Mutex<HotState>>,
        _base_layer: MapLayer<StateKey, StateSlot>,
        _top_layer: MapLayer<StateKey, StateSlot>,
        overlay: LayeredMap<StateKey, StateSlot>,
    }

    impl LRUTest {
        fn new_empty() -> Self {
            let hot_state = Arc::new(Mutex::new(HotState {
                inner: HashMap::new(),
                head: None,
                tail: None,
            }));
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

        fn commit_updates(
            &self,
            updates: HashMap<StateKey, StateSlot>,
            head: Option<StateKey>,
            tail: Option<StateKey>,
            num_items: usize,
        ) {
            let mut locked = self.hot_state.lock().unwrap();
            locked
                .inner
                .extend(updates.into_iter().filter(|(_key, slot)| slot.is_hot()));
            locked.head = head;
            locked.tail = tail;
            assert_eq!(locked.inner.len(), num_items);
        }
    }

    fn arb_state_slot() -> impl Strategy<Value = StateSlot> {
        any::<Version>().prop_flat_map(|hot_since_version| {
            prop_oneof![
                2 => Just(StateSlot::HotVacant {
                    hot_since_version,
                    lru_info: LRUEntry::uninitialized(),
                }),
                1 => Just(StateSlot::ColdVacant),
            ]
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]

        #[test]
        fn test_empty_overlay(
            (updates1, updates2) in hash_set(any::<StateKey>(), 1..20)
                .prop_flat_map(|keys| {
                    let pool: Vec<_> = keys.into_iter().collect();
                    (
                        vec((sample::select(pool.clone()), arb_state_slot()), 1..50),
                        vec((sample::select(pool), arb_state_slot()), 1..50),
                    )
                }),
        ) {
            let test_obj = LRUTest::new_empty();
            let mut lru = HotStateLRU::new(
                Arc::clone(&test_obj.hot_state) as Arc<dyn HotStateView>,
                &test_obj.overlay,
                None,
                None,
                0,
            );

            for (key, value) in &updates1 {
                if value.is_hot() {
                    lru.insert(key.clone(), value.clone());
                } else {
                    lru.evict(key);
                }
            }

            lru.validate_lru();
            let (updates, new_head, new_tail, new_num_items) = lru.into_updates();
            test_obj.commit_updates(updates, new_head.clone(), new_tail.clone(), new_num_items);

            let mut lru = HotStateLRU::new(
                Arc::clone(&test_obj.hot_state) as Arc<dyn HotStateView>,
                &test_obj.overlay,
                new_head,
                new_tail,
                new_num_items,
            );

            for (key, value) in &updates2 {
                if value.is_hot() {
                    lru.insert(key.clone(), value.clone());
                } else {
                    lru.evict(key);
                }
            }

            lru.validate_lru();
        }
    }
}
