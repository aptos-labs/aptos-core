// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_view::hot_state_view::HotStateView;
use aptos_experimental_layered_map::LayeredMap;
use aptos_types::state_store::{
    hot_state::{LRUEntry, SpeculativeLRUEntry},
    state_key::StateKey,
};
use std::{collections::HashMap, sync::Arc};

#[allow(dead_code)] // TODO(HotState): remove.
pub(crate) struct LRUUpdater {
    /// The entire hot state resulted from committed transactions.
    committed_hot_state: Arc<dyn HotStateView>,
    /// Additional entries resulted from previous speculative execution.
    overlay: Arc<LayeredMap<StateKey, SpeculativeLRUEntry<StateKey>>>,
    /// The new entries from the current execution.
    pending: HashMap<StateKey, SpeculativeLRUEntry<StateKey>>,
    /// Points to the latest entry. `None` if empty.
    head: Option<StateKey>,
    /// Points to the oldest entry. `None` if empty.
    tail: Option<StateKey>,
}

impl LRUUpdater {
    pub fn new(
        committed_hot_state: Arc<dyn HotStateView>,
        overlay: Arc<LayeredMap<StateKey, SpeculativeLRUEntry<StateKey>>>,
        head: Option<StateKey>,
        tail: Option<StateKey>,
    ) -> Self {
        Self {
            committed_hot_state,
            overlay,
            pending: HashMap::new(),
            head,
            tail,
        }
    }

    pub fn insert(&mut self, key: StateKey) {
        self.delete(&key);
        self.insert_as_head(key);
    }

    fn insert_as_head(&mut self, key: StateKey) {
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

    pub fn delete(&mut self, key: &StateKey) {
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

    fn get_entry(&self, key: &StateKey) -> Option<LRUEntry<StateKey>> {
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

        self.committed_hot_state.get_lru_entry(key)
    }

    fn expect_entry(&self, key: &StateKey) -> LRUEntry<StateKey> {
        self.get_entry(key).expect("Key must exist.")
    }
}

#[cfg(test)]
mod tests {}
