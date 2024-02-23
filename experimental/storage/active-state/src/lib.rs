// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(unused_variables)]

use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_logger::info;
use aptos_scratchpad::sparse_merkle::SparseMerkleTree;
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::state_store::{state_key::StateKey, state_value::StateValue};
use bytes::Bytes;
use dashmap::DashMap;
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

// Operation to update the LRUCache
pub enum Action {
    Update(ItemId, StateValue),
    Add(Vec<(StateKey, Option<StateValue>)>),
    EvictAndAdd(StateKey, StateValue),
    Evict(StateKey), // when a state key is deleted
}

struct ActiveState {
    items: Vec<Item>,
    used_slots_cnt: usize,
    existing_items: DashMap<StateKey, ItemId>,
    latest_item: Option<ItemId>,
    oldest_in_mem_item: Option<ItemId>,
    oldest_item: Option<ItemId>,
    smt: SparseMerkleTree<StateValue>,
    proof_reader: BasicProofReader,
    max_items: usize,
}

impl ActiveState {
    pub fn new(smt: SparseMerkleTree<StateValue>, max_items: usize) -> Self {
        ActiveState {
            items: Vec::with_capacity(max_items),
            used_slots_cnt: 0,
            existing_items: DashMap::new(),
            latest_item: None,
            oldest_in_mem_item: None,
            oldest_item: None,
            smt,
            proof_reader: BasicProofReader::new(),
            max_items,
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
        // get the proof and remove the leaf node from the smt
        let proof = self
            .smt
            .remove_leaf_node(key.hash())
            .map_err(|e| AptosDbError::Other(e.to_string()))?;
        self.proof_reader.add_proof(key.hash(), proof);
        Ok(())
    }

    fn generate_actions(&mut self, value_set: Vec<(StateKey, Option<StateValue>)>) -> Vec<Action> {
        let mut updates = Vec::new();
        for (key, value) in value_set {
            if value.is_none() {
                updates.push(Action::Evict(key));
                continue;
            }

            let item_id_opt = self.existing_items.get(&key);
            let action = if let Some(item_id) = item_id_opt {
                Action::Update(*item_id, value.unwrap())
            } else {
                assert!(
                    self.used_slots_cnt <= self.max_items,
                    "Exceed the max items limit"
                );
                if self.used_slots_cnt == self.max_items {
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
                }
            };
            updates.push(action);
        }
        updates
    }

    fn update_active_state_item(&mut self, item_id: ItemId, value: StateValue) -> Result<()> {
        if let Some(previous_item) = self.items[item_id].previous {
            self.items[previous_item].next = self.items[item_id].next;
        }
        if let Some(next_item) = self.items[item_id].next {
            self.items[next_item].previous = self.items[item_id].previous;
        }
        // update the item's next and previous item
        self.items[item_id].next = None;
        self.items[item_id].previous = self.latest_item;
        // make the item the latest item
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
                let empty_slot = self.latest_item.map_or(0, |id| id + 1);

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
                self.latest_item = Some(empty_slot);
                self.used_slots_cnt += 1;
                self.existing_items.insert(key, empty_slot);
                self.items.push(item);
            });
        Ok(())
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
                    let evicted_key = self.evict_and_add_item(key, value)?;
                    self.evict_from_smt(&evicted_key)?;
                    info!("Evict {:?} and add key", evicted_key.hash());
                },
                Action::Evict(key) => {
                    info!("Evict {:?}", &key.hash());
                    self.evict_from_smt(&key)?;
                    self.evict_item(key)?;
                },
            }
        }
        Ok(())
    }

    fn evict_item(&mut self, key: StateKey) -> Result<()> {
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
            // update the neighbors
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
            self.oldest_item = item.next;
            self.existing_items.remove(&old_key);

            // use the slot for the new key value pair
            item.inner = Some(ItemInner {
                key: key.clone(),
                value: Value::InMemory {
                    bytes: value.unpack().1,
                },
            });
            // also mark this item to be the latest item
            item.previous = self.latest_item;
            item.next = None;
            self.latest_item = Some(item.id);

            self.existing_items.insert(key, item.id);
            Ok(old_key)
        } else {
            Err(AptosDbError::NotFound("No item to evict".to_string()))
        }
    }

    pub fn get_current_smt(&self) -> SparseMerkleTree<StateValue> {
        self.smt.clone()
    }

    #[cfg(test)]
    pub fn return_items(&self) -> Vec<Item> {
        self.items.clone()
    }
}
