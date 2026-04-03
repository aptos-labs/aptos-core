// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::state_store::state_view::hot_state_view::HotStateView;
use aptos_crypto::HashValue;
use aptos_experimental_layered_map::LayeredMap;
use aptos_types::{
    state_store::{hot_state::THotStateSlot, state_key::StateKey, state_slot::StateSlot},
    transaction::Version,
};
use std::{collections::HashMap, num::NonZeroUsize, sync::Arc};

pub(crate) struct HotStateLRU<'a> {
    /// Max total number of items in the cache.
    capacity: NonZeroUsize,
    /// The entire committed hot state. While this contains all the shards, this struct is supposed
    /// to handle a single shard.
    committed: Arc<dyn HotStateView>,
    /// Additional entries resulted from previous speculative execution.
    overlay: &'a LayeredMap<HashValue, StateSlot>,
    /// The new entries from current execution.
    pending: HashMap<HashValue, StateSlot>,
    /// Points to the latest entry. `None` if empty.
    head: Option<HashValue>,
    /// Points to the oldest entry. `None` if empty.
    tail: Option<HashValue>,
    /// Total number of items.
    num_items: usize,
}

impl<'a> HotStateLRU<'a> {
    pub fn new(
        capacity: NonZeroUsize,
        committed: Arc<dyn HotStateView>,
        overlay: &'a LayeredMap<HashValue, StateSlot>,
        head: Option<HashValue>,
        tail: Option<HashValue>,
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

    /// Inserts a hot slot as the most recent entry. Returns the old `hot_since_version` if
    /// replacing an existing hot entry.
    pub fn insert(&mut self, key: &StateKey, mut slot: StateSlot) -> Option<Version> {
        assert!(
            slot.is_hot(),
            "Should not insert cold slots into hot state."
        );
        assert!(
            slot.state_key().is_none_or(|sk| key == sk),
            "Map key and embedded state_key must match."
        );
        // Ensure state_key is populated. Slots loaded from the hot state DB don't carry
        // the full key (only the hash), so patch it here while we have the key available.
        if slot.state_key().is_none() {
            slot.set_state_key(key.clone());
        }
        let key_hash = *key.crypto_hash_ref();
        let old_hot_since = self.delete(&key_hash).map(|s| s.expect_hot_since_version());
        if old_hot_since.is_none() {
            self.num_items += 1;
        }
        self.insert_as_head(key_hash, slot);
        old_hot_since
    }

    fn insert_as_head(&mut self, key_hash: HashValue, mut slot: StateSlot) {
        match self.head.take() {
            Some(head) => {
                let mut old_head_slot = self.expect_hot_slot(&head);
                old_head_slot.set_prev(Some(key_hash));
                slot.set_prev(None);
                slot.set_next(Some(head));
                self.pending.insert(head, old_head_slot);
                self.pending.insert(key_hash, slot);
                self.head = Some(key_hash);
            },
            None => {
                slot.set_prev(None);
                slot.set_next(None);
                self.pending.insert(key_hash, slot);
                self.head = Some(key_hash);
                self.tail = Some(key_hash);
            },
        }
    }

    /// Returns the list of entries evicted, beginning from the LRU.
    pub fn maybe_evict(&mut self) -> Vec<(HashValue, StateSlot)> {
        let mut current = match self.tail {
            Some(tail) => tail,
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
            let prev_key_hash = *slot
                .prev()
                .expect("There must be at least one newer entry (num_items > capacity >= 1).");
            evicted.push((current, slot.clone()));
            self.pending.insert(current, slot.to_cold());
            current = prev_key_hash;
            self.num_items -= 1;
        }
        evicted
    }

    /// Returns the deleted slot, or `None` if the key doesn't exist or is not hot.
    fn delete(&mut self, key_hash: &HashValue) -> Option<StateSlot> {
        // Fetch the slot corresponding to the given key. Note that `self.pending` and
        // `self.overlay` may contain cold slots, like the ones recently evicted, and we need to
        // ignore them.
        let old_slot = match self.get_slot(key_hash) {
            Some(slot) if slot.is_hot() => slot,
            _ => return None,
        };

        match old_slot.prev() {
            Some(prev_key_hash) => {
                let mut prev_slot = self.expect_hot_slot(prev_key_hash);
                prev_slot.set_next(old_slot.next().copied());
                self.pending.insert(*prev_key_hash, prev_slot);
            },
            None => {
                // There is no newer entry. The current key was the head.
                self.head = old_slot.next().copied();
            },
        }

        match old_slot.next() {
            Some(next_key_hash) => {
                let mut next_slot = self.expect_hot_slot(next_key_hash);
                next_slot.set_prev(old_slot.prev().copied());
                self.pending.insert(*next_key_hash, next_slot);
            },
            None => {
                // There is no older entry. The current key was the tail.
                self.tail = old_slot.prev().copied();
            },
        }

        Some(old_slot)
    }

    pub(crate) fn get_slot(&self, key_hash: &HashValue) -> Option<StateSlot> {
        if let Some(slot) = self.pending.get(key_hash) {
            return Some(slot.clone());
        }

        if let Some(slot) = self.overlay.get(key_hash) {
            return Some(slot);
        }

        self.committed.get_state_slot(key_hash)
    }

    fn expect_hot_slot(&self, key_hash: &HashValue) -> StateSlot {
        let slot = self
            .get_slot(key_hash)
            .expect("Given key is expected to exist.");
        assert!(slot.is_hot(), "Given key is expected to be hot.");
        slot
    }

    pub fn into_updates(
        self,
    ) -> (
        HashMap<HashValue, StateSlot>,
        Option<HashValue>,
        Option<HashValue>,
        usize,
    ) {
        (self.pending, self.head, self.tail, self.num_items)
    }

    #[cfg(test)]
    fn iter(&self) -> Iter<'_, '_> {
        Iter {
            current_key: self.head,
            lru: self,
        }
    }

    #[cfg(test)]
    fn validate(&self) {
        self.validate_impl(self.head, |slot| slot.next().copied());
        self.validate_impl(self.tail, |slot| slot.prev().copied());
    }

    #[cfg(test)]
    fn validate_impl(
        &self,
        start: Option<HashValue>,
        func: impl Fn(&StateSlot) -> Option<HashValue>,
    ) {
        let mut current = start;
        let mut num_visited = 0;
        while let Some(key_hash) = current {
            let slot = self.expect_hot_slot(&key_hash);
            num_visited += 1;
            current = func(&slot);
        }
        assert_eq!(num_visited, self.num_items);
    }
}

#[cfg(test)]
struct Iter<'a, 'b> {
    current_key: Option<HashValue>,
    lru: &'a HotStateLRU<'b>,
}

#[cfg(test)]
impl Iterator for Iter<'_, '_> {
    type Item = (HashValue, StateSlot);

    fn next(&mut self) -> Option<(HashValue, StateSlot)> {
        let key_hash = self.current_key.take()?;
        let slot = self.lru.expect_hot_slot(&key_hash);
        self.current_key = slot.next().copied();
        Some((key_hash, slot))
    }
}

#[cfg(test)]
mod tests {
    use super::HotStateLRU;
    use crate::state_store::state_view::hot_state_view::HotStateView;
    use aptos_crypto::HashValue;
    use aptos_experimental_layered_map::{LayeredMap, MapLayer};
    use aptos_types::{
        state_store::{
            hot_state::LRUEntry,
            state_key::StateKey,
            state_slot::{StateSlot, StateSlotKind},
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
        inner: HashMap<HashValue, StateSlot>,
        head: Option<HashValue>,
        tail: Option<HashValue>,
    }

    impl HotStateView for Mutex<HotState> {
        fn get_state_slot(&self, key_hash: &HashValue) -> Option<StateSlot> {
            self.lock().unwrap().inner.get(key_hash).cloned()
        }
    }

    struct LRUTest {
        hot_state: Arc<Mutex<HotState>>,
        _base_layer: MapLayer<HashValue, StateSlot>,
        _top_layer: MapLayer<HashValue, StateSlot>,
        overlay: LayeredMap<HashValue, StateSlot>,
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
            updates: HashMap<HashValue, StateSlot>,
            head: Option<HashValue>,
            tail: Option<HashValue>,
            num_items: usize,
        ) {
            // For most of the logic we don't really care whether the data is in committed state or
            // in the overlay, so we just merge everything directly to committed state, and keep
            // the overlay empty.
            let mut locked = self.hot_state.lock().unwrap();
            for (key_hash, slot) in updates {
                if slot.is_hot() {
                    locked.inner.insert(key_hash, slot);
                } else {
                    locked.inner.remove(&key_hash);
                }
            }
            locked.head = head;
            locked.tail = tail;
            assert_eq!(locked.inner.len(), num_items);
        }
    }

    fn arb_hot_slot_kind() -> impl Strategy<Value = StateSlotKind> {
        (any::<Version>(), any::<StateValue>()).prop_flat_map(|(version, value)| {
            prop_oneof![
                4 => Just(StateSlotKind::HotOccupied {
                    value_version: version,
                    value,
                    hot_since_version: version,
                    lru_info: LRUEntry::uninitialized(),
                }),
                1 => Just(StateSlotKind::HotVacant {
                    hot_since_version: version,
                    lru_info: LRUEntry::uninitialized(),
                }),
            ]
        })
    }

    fn assert_lru_equal(actual: &HotStateLRU, expected: &LruCache<HashValue, StateSlot>) {
        assert_eq!(
            actual
                .iter()
                .map(|(key_hash, slot)| {
                    assert!(slot.is_hot());
                    (key_hash, slot.into_state_value_opt())
                })
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|(key_hash, slot)| {
                    assert!(slot.is_hot());
                    (*key_hash, slot.clone().into_state_value_opt())
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
                        let block = vec((sample::select(pool.clone()), arb_hot_slot_kind()), 1..100);
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

                for (key, kind) in updates {
                    let key_hash = *key.crypto_hash_ref();
                    let slot = StateSlot::new(key.clone(), kind);
                    lru.insert(&key, slot.clone());
                    naive_lru.put(key_hash, slot);
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
                test_obj.commit_updates(updates, new_head, new_tail, new_num_items);
                head = new_head;
                tail = new_tail;
                num_items = new_num_items;
            }
        }
    }
}
