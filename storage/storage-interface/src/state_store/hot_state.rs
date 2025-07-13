// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_view::hot_state_view::HotStateView;
use aptos_experimental_layered_map::LayeredMap;
use aptos_types::state_store::{state_key::StateKey, state_slot::StateSlot};
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
        self.delete(&key);
        self.insert_as_head(key, slot);
    }

    fn insert_as_head(&mut self, key: StateKey, mut slot: StateSlot) {
        self.num_entries_changed += 1;
        match self.head.take() {
            Some(head) => {
                let mut old_head_slot = self.expect_slot(&head);
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
        self.num_entries_changed -= 1;

        match old_entry.prev() {
            Some(prev_key) => {
                let mut prev_entry = self.expect_slot(prev_key);
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
                let mut next_entry = self.expect_slot(next_key);
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

    fn expect_slot(&self, key: &StateKey) -> StateSlot {
        self.get_slot(key).expect("Given key is expected to exist.")
    }
}

#[cfg(test)]
mod tests {
    use super::HotStateLRU;
    use crate::state_store::state_view::hot_state_view::HotStateView;
    use aptos_experimental_layered_map::MapLayer;
    use aptos_types::state_store::{
        hot_state::{LRUEntry, SpeculativeLRUEntry},
        state_key::StateKey,
        state_slot::StateSlot,
    };
    use maplit::hashmap;
    use std::{collections::HashMap, sync::Arc};

    #[derive(Debug)]
    struct HotState {
        inner: HashMap<StateKey, StateSlot>,
        head: Option<StateKey>,
        tail: Option<StateKey>,
    }

    impl HotStateView for HotState {
        fn get_state_slot(&self, state_key: &StateKey) -> Option<StateSlot> {
            self.inner.get(state_key).map(|v| v.clone())
        }
    }

    struct LRUTest<'a> {
        _hot_state: Arc<HotState>,
        _base_layer: MapLayer<StateKey, StateSlot>,
        _top_layer: MapLayer<StateKey, StateSlot>,
        lru: HotStateLRU<'a>,
    }

    impl<'a> LRUTest<'a> {
        fn new_empty() -> Self {
            let hot_state = Arc::new(HotState {
                inner: HashMap::new(),
                head: None,
                tail: None,
            });
            let base_layer = MapLayer::new_family("test");
            let top_layer = base_layer.clone();
            let overlay = Arc::new(base_layer.view_layers_after(&top_layer));

            let lru = HotStateLRU::new(
                Arc::clone(&hot_state) as Arc<dyn HotStateView<Key = u32, Value = ()>>,
                overlay,
                hot_state.head,
                hot_state.tail,
            );

            Self {
                _hot_state: hot_state,
                _base_layer: base_layer,
                _top_layer: top_layer,
                lru,
            }
        }
    }

    #[test]
    fn test_empty_overlay() {
        let mut test_obj = LRUTest::new_empty();
        let lru = &mut test_obj.lru;
        assert_eq!(lru.head, None);
        assert_eq!(lru.tail, None);
        assert!(lru.pending.is_empty());

        lru.insert(1);
        assert_eq!(lru.head, Some(1));
        assert_eq!(lru.tail, Some(1));
        assert_eq!(
            lru.pending,
            hashmap! {1 => SpeculativeLRUEntry::Existing(LRUEntry { prev: None, next: None })}
        );

        lru.delete(&1);
        assert_eq!(lru.head, None);
        assert_eq!(lru.tail, None);
        assert_eq!(lru.pending, hashmap! {1 => SpeculativeLRUEntry::Deleted});

        lru.insert(1);
        lru.insert(2);
        lru.insert(3);
        assert_eq!(lru.head, Some(3));
        assert_eq!(lru.tail, Some(1));
        assert_eq!(lru.pending, hashmap! {
            1 => SpeculativeLRUEntry::Existing(LRUEntry { prev: Some(2), next: None }),
            2 => SpeculativeLRUEntry::Existing(LRUEntry { prev: Some(3), next: Some(1) }),
            3 => SpeculativeLRUEntry::Existing(LRUEntry { prev: None, next: Some(2) }),
        });

        lru.insert(2);
        assert_eq!(lru.head, Some(2));
        assert_eq!(lru.tail, Some(1));
        assert_eq!(lru.pending, hashmap! {
            1 => SpeculativeLRUEntry::Existing(LRUEntry { prev: Some(3), next: None }),
            2 => SpeculativeLRUEntry::Existing(LRUEntry { prev: None, next: Some(3) }),
            3 => SpeculativeLRUEntry::Existing(LRUEntry { prev: Some(2), next: Some(1) }),
        });

        lru.delete(&1);
        assert_eq!(lru.head, Some(2));
        assert_eq!(lru.tail, Some(3));
        assert_eq!(lru.pending, hashmap! {
            1 => SpeculativeLRUEntry::Deleted,
            2 => SpeculativeLRUEntry::Existing(LRUEntry { prev: None, next: Some(3) }),
            3 => SpeculativeLRUEntry::Existing(LRUEntry { prev: Some(2), next: None }),
        });
    }
}
