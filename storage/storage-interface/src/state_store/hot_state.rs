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
    /// Max of `total_bytes()`.
    max_bytes: usize,
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
    /// Sum of `StateSlot::size()` across all hot entries in this shard.
    total_value_bytes: usize,
}

impl<'a> HotStateLRU<'a> {
    pub fn new(
        capacity: NonZeroUsize,
        max_bytes: usize,
        committed: Arc<dyn HotStateView>,
        overlay: &'a LayeredMap<HashValue, StateSlot>,
        head: Option<HashValue>,
        tail: Option<HashValue>,
        num_items: usize,
        total_value_bytes: usize,
    ) -> Self {
        Self {
            capacity,
            max_bytes,
            committed,
            overlay,
            pending: HashMap::new(),
            head,
            tail,
            num_items,
            total_value_bytes,
        }
    }

    /// Takes effect at the next `maybe_evict`.
    pub fn set_limits(&mut self, capacity: NonZeroUsize, max_bytes: usize) {
        self.capacity = capacity;
        self.max_bytes = max_bytes;
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
        // Ensure state_key is populated. Slots from the hot state base don't carry the full key
        // (only the hash), so patch it here while we have the key available.
        if slot.state_key().is_none() {
            slot.set_state_key(key.clone());
        }
        let key_hash = *key.crypto_hash_ref();
        let old_slot = self.delete(&key_hash);
        match &old_slot {
            Some(old) => self.total_value_bytes -= old.size(),
            None => self.num_items += 1,
        }
        self.total_value_bytes += slot.size();
        self.insert_as_head(key_hash, slot);
        old_slot.map(|s| s.expect_hot_since_version())
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
        let mut evicted = Vec::new();
        // n.b. If the newest entry alone exceeds `max_bytes`, it gets evicted too, leaving the
        // cache empty.
        while self.num_items > self.capacity.get() || self.total_bytes() > self.max_bytes {
            let tail = self
                .tail
                .expect("There must be entries to evict when current size is above limit.");
            let slot = self.delete(&tail).expect("Tail must be hot.");
            self.total_value_bytes -= slot.size();
            evicted.push((tail, slot.clone()));
            self.pending.insert(tail, slot.to_cold());
            self.num_items -= 1;
        }
        evicted
    }

    /// Sum of key and value sizes, counting each key as its hash since that's all the hot state
    /// base keeps.
    fn total_bytes(&self) -> usize {
        self.num_items * HashValue::LENGTH + self.total_value_bytes
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
        usize,
    ) {
        (
            self.pending,
            self.head,
            self.tail,
            self.num_items,
            self.total_value_bytes,
        )
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
        let mut total_value_bytes = 0;
        while let Some(key_hash) = current {
            let slot = self.expect_hot_slot(&key_hash);
            num_visited += 1;
            total_value_bytes += slot.size();
            current = func(&slot);
        }
        assert_eq!(num_visited, self.num_items);
        assert_eq!(total_value_bytes, self.total_value_bytes);
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
    use crate::state_store::{
        leaf_entry::leaf_entry_to_jmt_update, state_view::hot_state_view::HotStateView,
    };
    use aptos_crypto::{hash::CryptoHash, HashValue};
    use aptos_experimental_layered_map::{LayeredMap, MapLayer};
    use aptos_types::{
        state_store::{
            hot_state::LRUEntry,
            state_key::StateKey,
            state_slot::{StateSlot, StateSlotKind},
            state_value::{StateValue, ARB_STATE_VALUE_MAX_SIZE},
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

        fn commit_updates(&self, updates: HashMap<HashValue, StateSlot>, num_items: usize) {
            // For most of the logic we don't really care whether the data is in committed state or
            // in the overlay, so we just merge everything directly to committed state, and keep
            // the overlay empty.
            let mut locked = self.hot_state.lock().unwrap();
            for (key_hash, mut slot) in updates {
                if slot.is_hot() {
                    // Like the real hot state base, keep only the key hash.
                    slot.clear_state_key();
                    locked.inner.insert(key_hash, slot);
                } else {
                    locked.inner.remove(&key_hash);
                }
            }
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

    /// Each entry is a 32-byte key hash plus a value of at most `ARB_STATE_VALUE_MAX_SIZE` bytes, so
    /// the byte limit often kicks in before the item limit, and can be below the size of a single
    /// entry.
    fn arb_max_bytes() -> impl Strategy<Value = usize> {
        prop_oneof![
            1 => Just(usize::MAX),
            3 => 0..=(HashValue::LENGTH + ARB_STATE_VALUE_MAX_SIZE) * 5,
        ]
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

    /// Runs one block against a fresh `HotStateLRU` built from `committed` and `overlay`: applies
    /// `updates`, evicts down to `capacity` and `max_bytes`, and checks every step against the
    /// `naive_lru` oracle. Returns `into_updates()` so the caller can commit or stack the result
    /// per its own model.
    fn run_block(
        capacity: NonZeroUsize,
        max_bytes: usize,
        committed: Arc<dyn HotStateView>,
        overlay: &LayeredMap<HashValue, StateSlot>,
        head: Option<HashValue>,
        tail: Option<HashValue>,
        num_items: usize,
        total_value_bytes: usize,
        updates: Vec<(StateKey, StateSlotKind)>,
        naive_lru: &mut LruCache<HashValue, StateSlot>,
    ) -> (
        HashMap<HashValue, StateSlot>,
        Option<HashValue>,
        Option<HashValue>,
        usize,
        usize,
    ) {
        let mut lru = HotStateLRU::new(
            capacity,
            max_bytes,
            committed,
            overlay,
            head,
            tail,
            num_items,
            total_value_bytes,
        );
        lru.validate();

        for (key, kind) in updates {
            let key_hash = *key.crypto_hash_ref();
            let slot = StateSlot::new(key.clone(), kind);
            lru.insert(&key, slot.clone());
            naive_lru.put(key_hash, slot);
            lru.validate();
            assert_lru_equal(&lru, naive_lru);
        }

        let actual_evicted = lru.maybe_evict();
        let mut expected_evicted = Vec::new();
        let mut naive_value_bytes: usize = naive_lru.iter().map(|(_, s)| s.size()).sum();
        while naive_lru.len() > capacity.get()
            || naive_lru.len() * HashValue::LENGTH + naive_value_bytes > max_bytes
        {
            let (key_hash, slot) = naive_lru.pop_lru().unwrap();
            naive_value_bytes -= slot.size();
            expected_evicted.push((key_hash, slot));
        }
        itertools::zip_eq(actual_evicted, expected_evicted).for_each(|(actual, expected)| {
            assert_eq!(actual.0, expected.0);
            assert_eq!(
                actual.1.into_state_value_opt(),
                expected.1.into_state_value_opt()
            );
        });
        lru.validate();
        assert_lru_equal(&lru, naive_lru);

        let (new_items, new_head, new_tail, new_num_items, new_total_value_bytes) =
            lru.into_updates();

        assert_eq!(new_head, naive_lru.peek_mru().map(|(k, _)| *k));
        assert_eq!(new_tail, naive_lru.peek_lru().map(|(k, _)| *k));
        assert_eq!(new_num_items, naive_lru.len());
        assert_eq!(
            new_total_value_bytes,
            naive_lru.iter().map(|(_, s)| s.size()).sum::<usize>(),
        );

        (
            new_items,
            new_head,
            new_tail,
            new_num_items,
            new_total_value_bytes,
        )
    }

    /// Mirrors the overlay model in `State::update`: the committed (persisted) hot state lags
    /// while speculatively executed blocks stack as overlay layers; `flush` is the background
    /// committer catching up and collapsing the overlay into the committed base.
    struct OverlayTest {
        committed: Arc<Mutex<HotState>>,
        /// Base of the current overlay view; advances to `top` on every flush.
        base: MapLayer<HashValue, StateSlot>,
        /// Top of the current overlay view.
        top: MapLayer<HashValue, StateSlot>,
        overlay: LayeredMap<HashValue, StateSlot>,
    }

    impl OverlayTest {
        fn new_empty() -> Self {
            let committed = Arc::new(Mutex::new(HotState {
                inner: HashMap::new(),
            }));
            let base = MapLayer::new_family("test");
            let top = base.clone();
            let overlay = top.view_layers_after(&base);
            Self {
                committed,
                base,
                top,
                overlay,
            }
        }

        fn committed_view(&self) -> Arc<dyn HotStateView> {
            Arc::clone(&self.committed) as Arc<dyn HotStateView>
        }

        /// Stacks one block's resulting slots as a new overlay layer, mirroring
        /// `overlay.new_layer(&new_items)` in `State::update`.
        fn stack(&mut self, updates: &[(HashValue, StateSlot)]) {
            self.top = self.overlay.new_layer(updates);
            self.overlay = self.top.view_layers_after(&self.base);
        }

        /// Collapses the overlay into the committed base. Hot slots are persisted without their
        /// `StateKey`; cold (evicted) slots are dropped, matching how the base only retains hot
        /// entries. The overlay carries the latest slot per key, so it is the flush set directly.
        fn flush(&mut self) {
            let mut committed = self.committed.lock().unwrap();
            for (key_hash, mut slot) in self.overlay.iter() {
                if slot.is_hot() {
                    slot.clear_state_key();
                    committed.inner.insert(key_hash, slot);
                } else {
                    committed.inner.remove(&key_hash);
                }
            }
            self.base = self.top.clone();
            self.overlay = self.top.view_layers_after(&self.base);
        }
    }

    /// Slots from the base carry no `StateKey`, and are never newer than the persisted snapshot.
    /// Touching them as LRU neighbors or evicting them must not produce JMT updates that need the
    /// full key, and re-inserting them must restore the key.
    #[test]
    fn test_slots_from_base_without_state_key() {
        let key_a = StateKey::raw(b"a");
        let key_b = StateKey::raw(b"b");
        let key_c = StateKey::raw(b"c");
        let key_d = StateKey::raw(b"d");
        let [a, b, c, d] = [&key_a, &key_b, &key_c, &key_d].map(|k| *k.crypto_hash_ref());
        let value = |s: &[u8]| StateValue::new_legacy(s.to_vec().into());
        let occupied = |version, value| StateSlotKind::HotOccupied {
            value_version: version,
            value,
            hot_since_version: version,
            lru_info: LRUEntry::uninitialized(),
        };

        // Persisted snapshot at version 9. Base from newest to oldest: a, b, c (vacant).
        let min_version = 10;
        let base_slot = |kind: StateSlotKind, prev, next| {
            StateSlot::new_without_state_key(kind.with_lru_info(LRUEntry { prev, next }))
        };
        let slot_a = base_slot(occupied(7, value(b"va")), None, Some(b));
        let slot_b = base_slot(occupied(8, value(b"vb")), Some(a), Some(c));
        let slot_c = base_slot(
            StateSlotKind::HotVacant {
                hot_since_version: 9,
                lru_info: LRUEntry::uninitialized(),
            },
            Some(b),
            None,
        );
        let total_value_bytes = slot_a.size() + slot_b.size();
        let test_obj = LRUTest::new_empty();
        test_obj.hot_state.lock().unwrap().inner =
            HashMap::from([(a, slot_a), (b, slot_b), (c, slot_c)]);

        let mut lru = HotStateLRU::new(
            NonZeroUsize::new(3).unwrap(),
            usize::MAX,
            Arc::clone(&test_obj.hot_state) as Arc<dyn HotStateView>,
            &test_obj.overlay,
            Some(a),
            Some(c),
            3,
            total_value_bytes,
        );
        // Writing d touches a as the old head.
        let value_d = value(b"vd");
        lru.insert(
            &key_d,
            StateSlot::new(key_d.clone(), occupied(10, value_d.clone())),
        );
        // Refresh b the way `State::apply_one_update` does, starting from the slot in the base.
        let mut refreshed_b = lru.get_slot(&b).unwrap();
        assert!(refreshed_b.state_key().is_none());
        refreshed_b.refresh(11);
        lru.insert(&key_b, refreshed_b);
        // c is the oldest, so it gets evicted.
        let evicted = lru.maybe_evict();
        assert_eq!(evicted.len(), 1);
        assert_eq!(evicted[0].0, c);
        lru.validate();
        let (updates, ..) = lru.into_updates();

        let jmt_updates: HashMap<_, _> = updates
            .iter()
            .filter(|(_, slot)| slot.passes_jmt_filter(min_version))
            .map(|(key_hash, slot)| leaf_entry_to_jmt_update(*key_hash, slot))
            .collect();
        // a: only its LRU pointers changed.
        assert!(updates[&a].state_key().is_none());
        assert!(!jmt_updates.contains_key(&a));
        // b: gets its key back, but the value didn't change.
        assert_eq!(updates[&b].state_key(), Some(&key_b));
        assert!(!jmt_updates.contains_key(&b));
        // c: the JMT deletion only needs the hash.
        assert!(updates[&c].is_cold());
        assert!(updates[&c].state_key().is_none());
        assert_eq!(jmt_updates[&c], None);
        // d: newly written.
        assert_eq!(jmt_updates[&d], Some((CryptoHash::hash(&value_d), key_d)));
        assert_eq!(jmt_updates.len(), 2);
    }

    fn hot_occupied(value_len: usize) -> StateSlotKind {
        StateSlotKind::HotOccupied {
            value_version: 0,
            value: StateValue::new_legacy(vec![0; value_len].into()),
            hot_since_version: 0,
            lru_info: LRUEntry::uninitialized(),
        }
    }

    fn hot_vacant() -> StateSlotKind {
        StateSlotKind::HotVacant {
            hot_since_version: 0,
            lru_info: LRUEntry::uninitialized(),
        }
    }

    /// Inserts `kinds` from oldest to newest into an empty LRU with room for plenty of items, then
    /// evicts down to `max_bytes`. Returns the indices of the evicted and the remaining entries,
    /// both from oldest to newest.
    fn evict_by_bytes(max_bytes: usize, kinds: Vec<StateSlotKind>) -> (Vec<usize>, Vec<usize>) {
        let test_obj = LRUTest::new_empty();
        let mut lru = HotStateLRU::new(
            NonZeroUsize::new(100).unwrap(),
            max_bytes,
            Arc::clone(&test_obj.hot_state) as Arc<dyn HotStateView>,
            &test_obj.overlay,
            None,
            None,
            0,
            0,
        );
        let keys: Vec<_> = (0..kinds.len())
            .map(|i| StateKey::raw(format!("key{i}").as_bytes()))
            .collect();
        for (key, kind) in keys.iter().zip(kinds) {
            lru.insert(key, StateSlot::new(key.clone(), kind));
        }
        let index_of = |key_hash: HashValue| {
            keys.iter()
                .position(|k| *k.crypto_hash_ref() == key_hash)
                .unwrap()
        };

        let evicted = lru
            .maybe_evict()
            .into_iter()
            .map(|(key_hash, _)| index_of(key_hash))
            .collect();
        lru.validate();
        // `iter()` starts from the newest.
        let mut remaining: Vec<_> = lru.iter().map(|(key_hash, _)| index_of(key_hash)).collect();
        remaining.reverse();
        (evicted, remaining)
    }

    #[test]
    fn test_max_bytes_counts_key_hashes() {
        // Each entry takes 32 + 10 = 42 bytes, so 3 entries take 126 bytes. The values alone
        // (30 bytes) would fit any of these limits.
        let entries = || vec![hot_occupied(10), hot_occupied(10), hot_occupied(10)];
        assert_eq!(evict_by_bytes(126, entries()), (vec![], vec![0, 1, 2]));
        assert_eq!(evict_by_bytes(125, entries()), (vec![0], vec![1, 2]));
        assert_eq!(evict_by_bytes(84, entries()), (vec![0], vec![1, 2]));
        assert_eq!(evict_by_bytes(83, entries()), (vec![0, 1], vec![2]));
    }

    #[test]
    fn test_max_bytes_counts_vacant_entries() {
        let entries = || vec![hot_vacant(), hot_vacant(), hot_vacant(), hot_vacant()];
        assert_eq!(evict_by_bytes(128, entries()), (vec![], vec![0, 1, 2, 3]));
        assert_eq!(evict_by_bytes(64, entries()), (vec![0, 1], vec![2, 3]));
    }

    #[test]
    fn test_max_bytes_mixed_entries() {
        // Sizes from oldest to newest: 32, 132, 32, 42.
        let entries = || {
            vec![
                hot_vacant(),
                hot_occupied(100),
                hot_vacant(),
                hot_occupied(10),
            ]
        };
        assert_eq!(evict_by_bytes(238, entries()), (vec![], vec![0, 1, 2, 3]));
        // Evicting the oldest vacant entry alone frees enough.
        assert_eq!(evict_by_bytes(206, entries()), (vec![0], vec![1, 2, 3]));
        assert_eq!(evict_by_bytes(205, entries()), (vec![0, 1], vec![2, 3]));
    }

    #[test]
    fn test_max_bytes_below_one_key_hash() {
        // Even a vacant entry doesn't fit, so the cache ends up empty.
        assert_eq!(
            evict_by_bytes(31, vec![hot_vacant(), hot_occupied(0)]),
            (vec![0, 1], vec![])
        );
        assert_eq!(
            evict_by_bytes(32, vec![hot_vacant(), hot_occupied(0)]),
            (vec![0], vec![1])
        );
    }

    proptest! {
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
            max_bytes in arb_max_bytes(),
        ) {
            let test_obj = LRUTest::new_empty();
            let mut head = None;
            let mut tail = None;
            let mut num_items = 0;
            let mut total_value_bytes = 0;
            // Use an unbounded cache because it can temporarily exceed the capacity in the middle
            // of the block.
            let mut naive_lru = LruCache::unbounded();

            for updates in blocks {
                let (new_items, new_head, new_tail, new_num_items, new_total_value_bytes) =
                    run_block(
                        capacity,
                        max_bytes,
                        Arc::clone(&test_obj.hot_state) as Arc<dyn HotStateView>,
                        &test_obj.overlay,
                        head,
                        tail,
                        num_items,
                        total_value_bytes,
                        updates,
                        &mut naive_lru,
                    );

                test_obj.commit_updates(new_items, new_num_items);
                head = new_head;
                tail = new_tail;
                num_items = new_num_items;
                total_value_bytes = new_total_value_bytes;
            }
        }

        /// Like `test_empty_overlay`, but each block's result is layered into a non-empty
        /// overlay (periodically flushed into committed). Exercises the
        /// `pending -> overlay -> committed` fall-through in `get_slot`, eviction walking the LRU
        /// chain through overlay-resident neighbors, and the cold-slots-in-overlay branch in
        /// `delete` — none of which the empty-overlay test can reach.
        #[test]
        fn test_nonempty_overlay(
            blocks_and_flushes in (hash_set(any::<StateKey>(), 1..50), 1..10usize)
                .prop_flat_map(|(keys, num_blocks)| {
                    let pool: Vec<_> = keys.into_iter().collect();
                    let blocks: Vec<_> = (0..num_blocks)
                        .map(|_| vec((sample::select(pool.clone()), arb_hot_slot_kind()), 1..100))
                        .collect();
                    (blocks, vec(any::<bool>(), num_blocks))
                }),
            capacity in (1..10usize).prop_map(|n| NonZeroUsize::new(n).unwrap()),
            max_bytes in arb_max_bytes(),
        ) {
            let (blocks, flushes) = blocks_and_flushes;
            let mut test_obj = OverlayTest::new_empty();
            let mut head = None;
            let mut tail = None;
            let mut num_items = 0;
            let mut total_value_bytes = 0;
            // Unbounded oracle: a block can temporarily exceed capacity before eviction.
            let mut naive_lru = LruCache::unbounded();

            for (updates, flush) in itertools::zip_eq(blocks, flushes) {
                let (new_items, new_head, new_tail, new_num_items, new_total_value_bytes) =
                    run_block(
                        capacity,
                        max_bytes,
                        test_obj.committed_view(),
                        &test_obj.overlay,
                        head,
                        tail,
                        num_items,
                        total_value_bytes,
                        updates,
                        &mut naive_lru,
                    );

                let new_items: Vec<_> = new_items.into_iter().collect();
                test_obj.stack(&new_items);
                if flush {
                    test_obj.flush();
                    // Once the committer fully catches up, committed holds exactly the live hot set.
                    assert_eq!(
                        test_obj.committed.lock().unwrap().inner.len(),
                        new_num_items,
                    );
                }

                head = new_head;
                tail = new_tail;
                num_items = new_num_items;
                total_value_bytes = new_total_value_bytes;
            }
        }
    }
}
