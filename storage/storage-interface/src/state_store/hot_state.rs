// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_view::hot_state_view::HotStateView;
use aptos_drop_helper::ArcAsyncDrop;
use aptos_experimental_layered_map::LayeredMap;
use aptos_types::state_store::hot_state::{LRUEntry, SpeculativeLRUEntry};
use std::{collections::HashMap, sync::Arc};

pub(crate) struct LRUUpdater<K, V>
where
    K: ArcAsyncDrop,
    V: ArcAsyncDrop,
{
    /// The entire hot state resulted from committed transactions.
    committed: Arc<dyn HotStateView<Key = K, Value = V>>,
    /// Additional entries resulted from previous speculative execution.
    overlay: Arc<LayeredMap<K, SpeculativeLRUEntry<K>>>,
    /// The new entries from the current execution.
    pub pending: HashMap<K, SpeculativeLRUEntry<K>>,
    /// Points to the latest entry. `None` if empty.
    pub head: Option<K>,
    /// Points to the oldest entry. `None` if empty.
    pub tail: Option<K>,
}

impl<K, V> LRUUpdater<K, V>
where
    K: ArcAsyncDrop + Clone + Eq + std::hash::Hash + Ord,
    V: ArcAsyncDrop,
{
    pub fn new(
        committed: Arc<dyn HotStateView<Key = K, Value = V>>,
        overlay: Arc<LayeredMap<K, SpeculativeLRUEntry<K>>>,
        head: Option<K>,
        tail: Option<K>,
    ) -> Self {
        Self {
            committed,
            overlay,
            pending: HashMap::new(),
            head,
            tail,
        }
    }

    pub fn insert(&mut self, key: K) {
        self.delete(&key);
        self.insert_as_head(key);
    }

    fn insert_as_head(&mut self, key: K) {
        match self.head.take() {
            Some(head) => {
                let mut old_head_entry = self.expect_entry(&head);
                old_head_entry.prev = Some(key.clone());
                let entry = LRUEntry {
                    prev: None,
                    next: Some(head.clone()),
                };
                self.pending
                    .insert(head, SpeculativeLRUEntry::Existing(old_head_entry));
                self.pending
                    .insert(key.clone(), SpeculativeLRUEntry::Existing(entry));
                self.head = Some(key);
            },
            None => {
                let entry = LRUEntry {
                    prev: None,
                    next: None,
                };
                self.pending
                    .insert(key.clone(), SpeculativeLRUEntry::Existing(entry));
                self.head = Some(key.clone());
                self.tail = Some(key);
            },
        }
    }

    pub fn delete(&mut self, key: &K) {
        let old_entry = match self.get_entry(key) {
            Some(e) => e,
            None => return,
        };

        match &old_entry.prev {
            Some(prev_key) => {
                let mut prev_entry = self.expect_entry(prev_key);
                prev_entry.next = old_entry.next.clone();
                self.pending
                    .insert(prev_key.clone(), SpeculativeLRUEntry::Existing(prev_entry));
            },
            None => {
                // There is no newer entry. The current key was the head.
                self.head = old_entry.next.clone();
            },
        }

        match &old_entry.next {
            Some(next_key) => {
                let mut next_entry = self.expect_entry(next_key);
                next_entry.prev = old_entry.prev;
                self.pending
                    .insert(next_key.clone(), SpeculativeLRUEntry::Existing(next_entry));
            },
            None => {
                // There is no older entry. The current key was the tail.
                self.tail = old_entry.prev;
            },
        }

        self.pending
            .insert(key.clone(), SpeculativeLRUEntry::Deleted);
    }

    fn get_entry(&self, key: &K) -> Option<LRUEntry<K>> {
        if let Some(entry) = self.pending.get(key) {
            match entry {
                SpeculativeLRUEntry::Existing(e) => return Some(e.clone()),
                SpeculativeLRUEntry::Deleted => return None,
            }
        }

        if let Some(entry) = self.overlay.get(key) {
            match entry {
                SpeculativeLRUEntry::Existing(e) => return Some(e),
                SpeculativeLRUEntry::Deleted => return None,
            }
        }

        self.committed.get_lru_entry(key)
    }

    fn expect_entry(&self, key: &K) -> LRUEntry<K> {
        self.get_entry(key).expect("Key must exist.")
    }
}

#[cfg(test)]
mod tests {
    use super::LRUUpdater;
    use crate::state_store::state_view::hot_state_view::HotStateView;
    use aptos_experimental_layered_map::MapLayer;
    use aptos_types::state_store::hot_state::{LRUEntry, SpeculativeLRUEntry};
    use maplit::hashmap;
    use std::{collections::HashMap, sync::Arc};

    #[derive(Debug)]
    struct HotState {
        inner: HashMap<u32, LRUEntry<u32>>,
        head: Option<u32>,
        tail: Option<u32>,
    }

    impl HotStateView for HotState {
        type Key = u32;
        type Value = ();

        fn get_state_slot(&self, state_key: &Self::Key) -> Option<Self::Value> {
            self.inner.get(state_key).map(|_| ())
        }

        fn get_lru_entry(&self, state_key: &Self::Key) -> Option<LRUEntry<Self::Key>> {
            self.inner.get(state_key).cloned()
        }
    }

    struct LRUUpdaterTest {
        _hot_state: Arc<HotState>,
        _base_layer: MapLayer<u32, SpeculativeLRUEntry<u32>>,
        _top_layer: MapLayer<u32, SpeculativeLRUEntry<u32>>,
        updater: LRUUpdater<u32, ()>,
    }

    impl LRUUpdaterTest {
        fn new_empty() -> Self {
            let hot_state = Arc::new(HotState {
                inner: HashMap::new(),
                head: None,
                tail: None,
            });
            let base_layer = MapLayer::new_family("test");
            let top_layer = base_layer.clone();
            let overlay = Arc::new(base_layer.view_layers_after(&top_layer));

            let updater = LRUUpdater::new(
                Arc::clone(&hot_state) as Arc<dyn HotStateView<Key = u32, Value = ()>>,
                overlay,
                hot_state.head,
                hot_state.tail,
            );

            Self {
                _hot_state: hot_state,
                _base_layer: base_layer,
                _top_layer: top_layer,
                updater,
            }
        }
    }

    #[test]
    fn test_empty_overlay() {
        let mut test_obj = LRUUpdaterTest::new_empty();
        let updater = &mut test_obj.updater;
        assert_eq!(updater.head, None);
        assert_eq!(updater.tail, None);
        assert!(updater.pending.is_empty());

        updater.insert(1);
        assert_eq!(updater.head, Some(1));
        assert_eq!(updater.tail, Some(1));
        assert_eq!(
            updater.pending,
            hashmap! {1 => SpeculativeLRUEntry::Existing(LRUEntry { prev: None, next: None })}
        );

        updater.delete(&1);
        assert_eq!(updater.head, None);
        assert_eq!(updater.tail, None);
        assert_eq!(
            updater.pending,
            hashmap! {1 => SpeculativeLRUEntry::Deleted}
        );

        updater.insert(1);
        updater.insert(2);
        updater.insert(3);
        assert_eq!(updater.head, Some(3));
        assert_eq!(updater.tail, Some(1));
        assert_eq!(updater.pending, hashmap! {
            1 => SpeculativeLRUEntry::Existing(LRUEntry { prev: Some(2), next: None }),
            2 => SpeculativeLRUEntry::Existing(LRUEntry { prev: Some(3), next: Some(1) }),
            3 => SpeculativeLRUEntry::Existing(LRUEntry { prev: None, next: Some(2) }),
        });

        updater.insert(2);
        assert_eq!(updater.head, Some(2));
        assert_eq!(updater.tail, Some(1));
        assert_eq!(updater.pending, hashmap! {
            1 => SpeculativeLRUEntry::Existing(LRUEntry { prev: Some(3), next: None }),
            2 => SpeculativeLRUEntry::Existing(LRUEntry { prev: None, next: Some(3) }),
            3 => SpeculativeLRUEntry::Existing(LRUEntry { prev: Some(2), next: Some(1) }),
        });

        updater.delete(&1);
        assert_eq!(updater.head, Some(2));
        assert_eq!(updater.tail, Some(3));
        assert_eq!(updater.pending, hashmap! {
            1 => SpeculativeLRUEntry::Deleted,
            2 => SpeculativeLRUEntry::Existing(LRUEntry { prev: None, next: Some(3) }),
            3 => SpeculativeLRUEntry::Existing(LRUEntry { prev: Some(2), next: None }),
        });
    }
}
