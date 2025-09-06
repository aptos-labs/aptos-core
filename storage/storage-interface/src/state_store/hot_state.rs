// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_view::hot_state_view::HotStateView;
use aptos_experimental_layered_map::LayeredMap;
use aptos_types::state_store::{
    hot_state::THotStateSlot, state_key::StateKey, state_slot::StateSlot,
};
use std::{collections::HashMap, num::NonZeroUsize, sync::Arc};

pub(crate) struct HotStateLRU<'a> {
    /// Max total number of items in the cache.
    capacity: NonZeroUsize,
    /// The entire committed hot state. While this contains all the shards, this struct is supposed
    /// to handle a single shard.
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
        capacity: NonZeroUsize,
        committed: Arc<dyn HotStateView>,
        overlay: &'a LayeredMap<StateKey, StateSlot>,
        head: Option<StateKey>,
        tail: Option<StateKey>,
        num_items: usize,
    ) -> Self {
        Self {
            capacity,
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
        if self.delete(&key).is_none() {
            self.num_items += 1;
        }
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
    }

    /// Returns the list of entries evicted, beginning from the LRU.
    pub fn maybe_evict(&mut self) -> Vec<(StateKey, StateSlot)> {
        let mut current = match &self.tail {
            Some(tail) => tail.clone(),
            None => {
                assert_eq!(self.num_items, 0);
                return Vec::new();
            },
        };

        let mut evicted = Vec::new();
        while self.num_items > self.capacity.get() {
            let slot = self
                .delete(&current)
                .expect("There must be entries to evict when current size is above capacity.");
            let prev_key = slot
                .prev()
                .cloned()
                .expect("There must be at least one newer entry (num_items > capacity >= 1).");
            evicted.push((current.clone(), slot.clone()));
            self.pending.insert(current, slot.to_cold());
            current = prev_key;
            self.num_items -= 1;
        }
        evicted
    }

    /// Returns the deleted slot, or `None` if the key doesn't exist or is not hot.
    fn delete(&mut self, key: &StateKey) -> Option<StateSlot> {
        // Fetch the slot corresponding to the given key. Note that `self.pending` and
        // `self.overlay` may contain cold slots, like the ones recently evicted, and we need to
        // ignore them.
        let old_slot = match self.get_slot(key) {
            Some(slot) if slot.is_hot() => slot,
            _ => return None,
        };

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

        Some(old_slot)
    }

    pub fn get_slot(&self, key: &StateKey) -> Option<StateSlot> {
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
        assert!(slot.is_hot(), "Given key is expected to be hot.");
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
        prelude::*,
        sample,
        strategy::Strategy,
    };
    use std::{
        collections::HashMap,
        num::NonZeroUsize,
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
            // For most of the logic we don't really care whether the data is in committed state or
            // in the overlay, so we just merge everything directly to committed state, and keep
            // the overlay empty.
            let mut locked = self.hot_state.lock().unwrap();
            for (key, slot) in updates {
                if slot.is_hot() {
                    locked.inner.insert(key, slot);
                } else {
                    locked.inner.remove(&key);
                }
            }
            locked.head = head;
            locked.tail = tail;
            assert_eq!(locked.inner.len(), num_items);
        }
    }

    fn arb_hot_slot() -> impl Strategy<Value = StateSlot> {
        (any::<Version>(), any::<StateValue>()).prop_flat_map(|(version, value)| {
            prop_oneof![
                4 => Just(StateSlot::HotOccupied {
                    value_version: version,
                    value,
                    hot_since_version: version,
                    lru_info: LRUEntry::uninitialized(),
                }),
                1 => Just(StateSlot::HotVacant {
                    hot_since_version: version,
                    lru_info: LRUEntry::uninitialized(),
                }),
            ]
        })
    }

    fn assert_lru_equal(actual: &HotStateLRU, expected: &LruCache<StateKey, StateSlot>) {
        assert_eq!(
            actual
                .iter()
                .map(|(key, slot)| {
                    assert!(slot.is_hot());
                    (key, slot.into_state_value_opt())
                })
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|(key, slot)| {
                    assert!(slot.is_hot());
                    (key.clone(), slot.clone().into_state_value_opt())
                })
                .collect::<Vec<_>>(),
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        #[test]
        fn test_empty_overlay(
            blocks in (hash_set(any::<StateKey>(), 1..50), 1..10usize)
                .prop_flat_map(|(keys, num_blocks)| {
                    let pool: Vec<_> = keys.into_iter().collect();
                    let mut blocks = Vec::new();
                    for _i in 0..num_blocks {
                        let block = vec((sample::select(pool.clone()), arb_hot_slot()), 1..100);
                        blocks.push(block);
                    }
                    blocks
                }),
            capacity in (1..10usize).prop_map(|n| NonZeroUsize::new(n).unwrap()),
        ) {
            let test_obj = LRUTest::new_empty();
            let mut head = None;
            let mut tail = None;
            let mut num_items = 0;
            // Use an unbounded cache because it can temporarily exceed the capacity in the middle
            // of the block.
            let mut naive_lru = LruCache::unbounded();

            for updates in blocks {
                let mut lru = HotStateLRU::new(
                    capacity,
                    Arc::clone(&test_obj.hot_state) as Arc<dyn HotStateView>,
                    &test_obj.overlay,
                    head,
                    tail,
                    num_items,
                );
                lru.validate();

                for (key, slot) in updates {
                    lru.insert(key.clone(), slot.clone());
                    naive_lru.put(key, slot);
                    lru.validate();
                    assert_lru_equal(&lru, &naive_lru);
                }

                let actual_evicted = lru.maybe_evict();
                let mut expected_evicted = Vec::new();
                while naive_lru.len() > capacity.get() {
                    expected_evicted.push(naive_lru.pop_lru().unwrap());
                }
                itertools::zip_eq(actual_evicted, expected_evicted).for_each(|(actual, expected)| {
                    assert_eq!(actual.0, expected.0);
                    assert_eq!(actual.1.into_state_value_opt(), expected.1.into_state_value_opt());
                });
                lru.validate();
                assert_lru_equal(&lru, &naive_lru);

                let (updates, new_head, new_tail, new_num_items) = lru.into_updates();
                test_obj.commit_updates(updates, new_head.clone(), new_tail.clone(), new_num_items);
                head = new_head;
                tail = new_tail;
                num_items = new_num_items;
            }
        }
    }
}
