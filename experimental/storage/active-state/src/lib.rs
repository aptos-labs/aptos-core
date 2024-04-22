// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_variables)]
#![allow(dead_code)]

use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_experimental_scratchpad::sparse_merkle::SparseMerkleTree;
use aptos_logger::info;
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::state_store::{state_key::StateKey, state_value::StateValue};
use bytes::Bytes;
use std::collections::{HashMap, VecDeque};
mod committer;
mod executor;
mod generator;
mod metrics;
pub mod pipeline;
#[cfg(test)]
mod tests;
mod utils;
use crate::utils::BasicProofReader;

pub const MAX_ITEMS: usize = 1 << 26; // about 64M leaf nodes
const ITEM_SIZE: usize = 48;
const MAX_BYTES: usize = 1 << 35; // 32 GB
const MAX_BYTES_PER_ITEM: usize = 1 << 10; // 1KB per item

// index of the the item in the vector
type ItemId = usize;

#[derive(Clone, Debug)]
struct ItemInner {
    key: StateKey,
    value: Value,
}

impl ItemInner {
    pub fn set_value(&mut self, value: StateValue) {
        let data = value.unpack().1;
        self.value = Value::InMemory { bytes: data };
    }

    pub fn get_value(&self) -> Value {
        self.value.clone()
    }

    pub fn get_key(&self) -> StateKey {
        self.key.clone()
    }
}

#[derive(Clone, Debug)]
struct Item {
    id: ItemId,
    inner: Option<ItemInner>,
    previous: Option<ItemId>,
    next: Option<ItemId>,
}

#[derive(Clone, Debug)]
enum Value {
    InMemory { bytes: Bytes },
    OnDisk { size: u16 },
}

impl Value {
    pub fn get_bytes(&self) -> Bytes {
        match self {
            Value::InMemory { bytes } => bytes.clone(),
            Value::OnDisk { size: _ } => Bytes::new(),
        }
    }
}

// Operation to update the LRUCache
pub enum Action {
    Update(ItemId, StateValue),
    Add(Vec<(StateKey, Option<StateValue>)>),
    EvictAndAdd(StateKey, StateValue),
    Delete(StateKey),
}

struct ActiveState {
    items: Vec<Item>,
    empty_slots: VecDeque<ItemId>, // we use this to track available slots after the cache is full
    existing_items: HashMap<StateKey, ItemId>,
    latest_item: Option<ItemId>,
    oldest_in_mem_item: Option<ItemId>,
    oldest_item: Option<ItemId>,
    smt: SparseMerkleTree<StateValue>,
    proof_reader: BasicProofReader,
    max_items: usize,
    used_slots_cnt: usize,
}

impl ActiveState {
    pub fn new(smt: SparseMerkleTree<StateValue>, max_items: usize) -> Self {
        ActiveState {
            items: Vec::with_capacity(max_items),
            empty_slots: VecDeque::new(),
            existing_items: HashMap::new(),
            latest_item: None,
            oldest_in_mem_item: None,
            oldest_item: None,
            smt,
            proof_reader: BasicProofReader::new(),
            max_items,
            used_slots_cnt: 0,
        }
    }

    pub fn add_to_smt(&mut self, updates: Vec<(HashValue, Option<&StateValue>)>) -> Result<()> {
        let new_smt = self
            .smt
            .batch_update_with_merge(updates, &self.proof_reader)
            .map_err(|e| AptosDbError::Other(e.to_string()))?;
        self.smt = new_smt;
        Ok(())
    }

    pub fn evict_from_smt(&mut self, key: &StateKey) -> Result<()> {
        // get the proof and we later use rely on the proof reader to mark node as unknown in smt
        let proof = self
            .smt
            .get_evicted_leaf_proof(key.hash())
            .map_err(|e| AptosDbError::Other(e.to_string()))?;
        self.proof_reader.add_proof(key.hash(), proof);
        self.add_to_smt(vec![(key.hash(), None)])?;
        Ok(())
    }

    fn generate_actions(&mut self, value_set: Vec<(StateKey, Option<StateValue>)>) -> Vec<Action> {
        let mut updates = Vec::new();
        for (key, value) in value_set {
            if value.is_none() {
                updates.push(Action::Delete(key));
                continue;
            }

            let item_id_opt = self.existing_items.get(&key);
            let action = if let Some(item_id) = item_id_opt {
                Action::Update(*item_id, value.unwrap())
            } else if self.used_slots_cnt == self.max_items {
                Action::EvictAndAdd(key, value.unwrap())
            } else {
                match updates.last_mut() {
                    Some(Action::Add(values)) => {
                        values.push((key, value));
                        // no need to add a new action after combining the updates
                        continue;
                    },
                    _ => Action::Add(vec![(key, value)]),
                }
            };
            updates.push(action);
        }
        updates
    }

    pub fn batch_put_value_set(
        &mut self,
        value_set: Vec<(StateKey, Option<StateValue>)>,
    ) -> Result<()> {
        let updates = self.generate_actions(value_set);
        for action in updates {
            match action {
                Action::Update(item_id, value) => {
                    self.update_active_state_item(item_id, value)?;
                    info!("Update item {:?}", item_id)
                },
                Action::Add(pairs) => {
                    info!("Add {} items", pairs.len());
                    self.add_items_to_active_state(pairs)?;
                },
                Action::EvictAndAdd(key, value) => {
                    self.add_to_smt(vec![(key.hash(), Some(&value))])?;
                    let evicted_key = self.evict_and_add_item(key, value)?;
                    self.evict_from_smt(&evicted_key)?;
                    info!("Evict {:?} and add key", evicted_key.hash());
                },
                Action::Delete(key) => {
                    info!("delete {:?}", &key.hash());
                    self.add_to_smt(vec![(key.hash(), None)])?;
                    self.delete_item(key)?;
                },
            }
        }
        Ok(())
    }

    fn update_active_state_item(&mut self, item_id: ItemId, value: StateValue) -> Result<()> {
        // update existing neighbors's link
        if let Some(previous_item) = self.items[item_id].previous {
            self.items[previous_item].next = self.items[item_id].next;
        }
        if let Some(next_item) = self.items[item_id].next {
            self.items[next_item].previous = self.items[item_id].previous;
        }

        // update the oldest item if item_id is the oldest item
        if self.oldest_item == Some(item_id) {
            self.oldest_item = self.items[item_id].next;
        }

        // update the item's next and previous item
        self.items[item_id].next = None;
        self.items[item_id].previous = self.latest_item;

        // make the item the latest item and update link
        if let Some(latest_item) = self.latest_item {
            self.items[latest_item].next = Some(item_id);
        }
        self.latest_item = Some(item_id);
        let item_inner = self
            .items
            .get_mut(item_id)
            .expect("updated item should exist")
            .inner
            .as_mut()
            .expect("inner should exist");
        item_inner.set_value(value);
        Ok(())
    }

    fn add_items_to_active_state(
        &mut self,
        pairs: Vec<(StateKey, Option<StateValue>)>,
    ) -> Result<()> {
        self.add_to_smt(
            pairs
                .iter()
                .map(|(key, value)| (key.hash(), value.as_ref()))
                .collect(),
        )?;

        pairs
            .into_iter()
            .filter(|(_, value)| value.is_some())
            .for_each(|(key, value)| {
                let empty_slot = self.get_empty_slot();

                let (_, bytes) = value.unwrap().unpack();
                let item = Item {
                    id: empty_slot,
                    inner: Some(ItemInner {
                        key: key.clone(),
                        value: Value::InMemory { bytes },
                    }),
                    previous: self.latest_item,
                    next: None,
                };
                // add to neighbors
                if let Some(previous_item) = item.previous {
                    self.items[previous_item].next = Some(empty_slot);
                }

                // update the latest item
                self.latest_item = Some(empty_slot);

                // update the oldest item
                if self.used_slots_cnt == 0 {
                    self.oldest_item = Some(empty_slot);
                }

                self.used_slots_cnt += 1;
                self.existing_items.insert(key, empty_slot);
                self.items.push(item);
            });
        Ok(())
    }

    fn delete_item(&mut self, key: StateKey) -> Result<()> {
        if let Some(item_id) = self.existing_items.get(&key) {
            let item = self.items.get_mut(*item_id).expect("Item not found");
            let previous_item = item.previous;
            let next_item = item.next;
            if let Some(previous_item) = previous_item {
                self.items[previous_item].next = next_item;
            }
            if let Some(next_item) = next_item {
                self.items[next_item].previous = previous_item;
            }
            // update the oldest item
            if self.oldest_item == Some(*item_id) {
                self.oldest_item = next_item;
            }

            // update the latest item
            if self.latest_item == Some(*item_id) {
                self.latest_item = previous_item;
            }

            self.empty_slots.push_back(*item_id);
            self.existing_items.remove(&key);
            self.used_slots_cnt -= 1;
            Ok(())
        } else {
            Err(AptosDbError::NotFound("Item not found".to_string()))
        }
    }

    // return the key of the evicted item
    fn evict_and_add_item(&mut self, key: StateKey, value: StateValue) -> Result<StateKey> {
        // evict the oldest item
        if let Some(oldest_item) = self.oldest_item {
            // cut the element from its neighbors
            let item = self
                .items
                .get_mut(oldest_item)
                .expect("Oldest item not found");
            let old_key = item
                .inner
                .as_ref()
                .expect("Oldest item has no inner")
                .key
                .clone();

            // update the oldest item
            self.oldest_item = item.next;
            self.existing_items.remove(&old_key);

            // use the slot for the new key value pair
            item.inner = Some(ItemInner {
                key: key.clone(),
                value: Value::InMemory {
                    bytes: value.unpack().1,
                },
            });
            // make the element the latest item
            item.previous = self.latest_item;
            item.next = None;
            let item_id = item.id;
            if let Some(latest_item) = self.latest_item {
                self.items[latest_item].next = Some(item_id);
            }
            self.latest_item = Some(item_id);

            // remove the old key from the existing items
            self.existing_items.insert(key, item_id);
            Ok(old_key)
        } else {
            Err(AptosDbError::NotFound("No item to evict".to_string()))
        }
    }

    fn get_empty_slot(&mut self) -> ItemId {
        if self.items.len() < self.max_items {
            self.latest_item.map_or(0, |id| id + 1)
        } else {
            self.empty_slots
                .pop_front()
                .expect("Empty slots should not be empty")
        }
    }

    pub fn get_current_smt(&self) -> SparseMerkleTree<StateValue> {
        self.smt.clone()
    }

    pub fn get_oldest_item(&self) -> Option<ItemId> {
        self.oldest_item
    }

    pub fn get_latest_item(&self) -> Option<ItemId> {
        self.latest_item
    }

    pub fn get_used_slots_cnt(&self) -> usize {
        self.used_slots_cnt
    }

    #[cfg(test)]
    pub fn return_items(&self) -> Vec<Item> {
        self.items.clone()
    }
}
