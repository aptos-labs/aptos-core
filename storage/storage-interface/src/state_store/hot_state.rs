// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_view::hot_state_view::HotStateView;
use aptos_experimental_layered_map::LayeredMap;
use aptos_logger::prelude::*;
use aptos_types::state_store::{
    hot_state::THotStateSlot, state_key::StateKey, state_slot::StateSlot,
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

/// NOTE: this always operates on a single shard.
pub(crate) struct HotStateLRU<'a> {
    shard_id: usize,
    /// The entire committed hot state. While this may contain all the shards, this struct is
    /// supposed to handle a single shard.
    committed: Arc<dyn HotStateView>,
    /// Additional entries resulted from previous speculative execution.
    overlay: &'a LayeredMap<StateKey, StateSlot>,
    /// The new entries from current execution.
    pending: HashMap<StateKey, StateSlot>,
    /// Points to the latest entry. `None` if empty.
    head: Option<StateKey>,
    /// Points to the oldest entry. `None` if empty.
    tail: Option<StateKey>,
    /// Total number of items.
    num_items: usize,
}

impl<'a> HotStateLRU<'a> {
    pub fn new(
        shard_id: usize,
        committed: Arc<dyn HotStateView>,
        overlay: &'a LayeredMap<StateKey, StateSlot>,
        head: Option<StateKey>,
        tail: Option<StateKey>,
        num_items: usize,
    ) -> Self {
        Self {
            shard_id,
            committed,
            overlay,
            pending: HashMap::new(),
            head,
            tail,
            num_items,
        }
    }

    pub fn insert(&mut self, key: StateKey, slot: StateSlot) {
        assert!(
            slot.is_hot(),
            "Should not insert cold slots into hot state."
        );
        self.delete(&key);
        self.insert_as_head(key, slot);
    }

    fn insert_as_head(&mut self, key: StateKey, mut slot: StateSlot) {
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

        info!("self.num_items: {} (before)", self.num_items);
        self.num_items += 1;
    }

    pub fn evict(&mut self, key: &StateKey) {
        if let Some(slot) = self.delete(key) {
            self.pending.insert(key.clone(), slot.to_cold());
        }
    }

    /// Returns the deleted slot.
    fn delete(&mut self, key: &StateKey) -> Option<StateSlot> {
        let old_slot = match self.get_slot(key) {
            Some(slot) if slot.is_hot() => slot,
            _ => return None,
        };
        info!("shard_id: {}, old_slot: {:?}", self.shard_id, old_slot);

        match old_slot.prev() {
            Some(prev_key) => {
                let mut prev_slot = self.expect_hot_slot(prev_key);
                prev_slot.set_next(old_slot.next().cloned());
                self.pending.insert(prev_key.clone(), prev_slot);
            },
            None => {
                // There is no newer entry. The current key was the head.
                self.head = old_slot.next().cloned();
            },
        }

        match old_slot.next() {
            Some(next_key) => {
                let mut next_slot = self.expect_hot_slot(next_key);
                next_slot.set_prev(old_slot.prev().cloned());
                self.pending.insert(next_key.clone(), next_slot);
            },
            None => {
                // There is no older entry. The current key was the tail.
                self.tail = old_slot.prev().cloned();
            },
        }

        info!("self.num_items: {} (before)", self.num_items);
        self.num_items -= 1;
        Some(old_slot)
    }

    fn get_slot(&self, key: &StateKey) -> Option<StateSlot> {
        if let Some(slot) = self.pending.get(key) {
            return Some(slot.clone());
        }

        if let Some(slot) = self.overlay.get(key) {
            return Some(slot);
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
        (
            self.pending.into_iter().collect(),
            self.head,
            self.tail,
            self.num_items,
        )
    }

    #[cfg(test)]
    fn iter(&self) -> Iter {
        Iter {
            current_key: self.head.clone(),
            lru: self,
        }
    }

    #[cfg(test)]
    fn validate(&self) {
        self.validate_impl(self.head.clone(), |slot| slot.next());
        self.validate_impl(self.tail.clone(), |slot| slot.prev());
    }

    #[cfg(test)]
    fn validate_impl(
        &self,
        start: Option<StateKey>,
        func: impl Fn(&StateSlot) -> Option<&StateKey>,
    ) {
        let mut current = start;
        let mut num_visited = 0;
        while let Some(key) = current {
            let slot = self.expect_hot_slot(&key);
            num_visited += 1;
            current = func(&slot).cloned();
        }
        assert_eq!(num_visited, self.num_items);
    }
}

#[cfg(test)]
struct Iter<'a, 'b> {
    current_key: Option<StateKey>,
    lru: &'a HotStateLRU<'b>,
}

#[cfg(test)]
impl<'a, 'b> Iterator for Iter<'a, 'b> {
    type Item = (StateKey, StateSlot);

    fn next(&mut self) -> Option<(StateKey, StateSlot)> {
        let key = self.current_key.take()?;
        let slot = self.lru.expect_hot_slot(&key);
        self.current_key = slot.next().cloned();
        Some((key, slot))
    }
}

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
        collection::{hash_set, vec},
        option,
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

    fn arb_state_slot() -> impl Strategy<Value = Option<StateSlot>> {
        (any::<Version>(), any::<StateValue>()).prop_flat_map(|(version, value)| {
            option::weighted(
                0.8,
                Just(StateSlot::HotOccupied {
                    value_version: version,
                    value,
                    hot_since_version: version,
                    lru_info: LRUEntry::uninitialized(),
                }),
            )
        })
    }

    fn assert_lru_equal(actual: &HotStateLRU, expected: &LruCache<StateKey, StateSlot>) {
        assert_eq!(
            actual
                .iter()
                .map(|(key, slot)| (key, slot.into_state_value_opt().unwrap()))
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|(key, slot)| (key.clone(), slot.clone().into_state_value_opt().unwrap()))
                .collect::<Vec<_>>(),
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        #[test]
        fn test_empty_overlay(
            (updates1, updates2) in hash_set(any::<StateKey>(), 1..50)
                .prop_flat_map(|keys| {
                    let pool: Vec<_> = keys.into_iter().collect();
                    (
                        vec((sample::select(pool.clone()), arb_state_slot()), 1..100),
                        vec((sample::select(pool), arb_state_slot()), 1..100),
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
            lru.validate();

            let mut naive_lru = LruCache::unbounded();
            for (key, slot_opt) in updates1 {
                match slot_opt {
                    Some(slot) => {
                        lru.insert(key.clone(), slot.clone());
                        naive_lru.put(key, slot);
                    }
                    None => {
                        lru.evict(&key);
                        naive_lru.pop(&key);
                    }
                }
                assert_lru_equal(&lru, &naive_lru);
            }
            lru.validate();
            // TODO: maybe verify the content of pending (including the cold slots) is expected?

            let (updates, new_head, new_tail, new_num_items) = lru.into_updates();
            test_obj.commit_updates(updates, new_head.clone(), new_tail.clone(), new_num_items);
            let mut lru = HotStateLRU::new(
                Arc::clone(&test_obj.hot_state) as Arc<dyn HotStateView>,
                &test_obj.overlay,
                new_head,
                new_tail,
                new_num_items,
            );
            lru.validate();

            for (key, slot_opt) in updates2 {
                match slot_opt {
                    Some(slot) => {
                        lru.insert(key.clone(), slot.clone());
                        naive_lru.put(key, slot);
                    }
                    None => {
                        lru.evict(&key);
                        naive_lru.pop(&key);
                    }
                }
                assert_lru_equal(&lru, &naive_lru);
            }
            lru.validate();
        }
    }
}
