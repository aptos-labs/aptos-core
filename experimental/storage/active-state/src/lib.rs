// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(unused_variables)]
use crate::atomic_bitmap::AtomicBitmap;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_storage_interface::Result;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use bytes::Bytes;
use dashmap::DashMap;
use std::sync::{
    atomic::{AtomicU32, AtomicU64, Ordering},
    mpsc::{channel, Receiver, Sender},
    Arc,
};

use aptos_scratchpad::sparse_merkle::node::NodeHandle;
#[cfg(test)]
pub mod tests;

const MAX_ITEMS: usize = 1 << 26; // about 64M leaf nodes
const ITEM_SIZE: usize = 48;
const MAX_BYTES: usize = 1 << 35; // 32 GB
const MAX_BYTES_PER_ITEM: usize = 1 << 10; // 1KB per item

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct StateKeyHash(HashValue);

// index of the the item in the vector
type ItemId = u32;

struct Item {
    id: ItemId,
    node: NodeHandle<StateValue>, // a refence to the node in SMT
    previous: Option<ItemId>,
    next: Option<ItemId>,
}

enum Value {
    InMemory { bytes: Bytes },
    OnDisk { size: u16 },
}

struct ActiveState {
    items: Vec<Item>,
    used_slots_cnt: AtomicU32,
    existing_items: DashMap<StateKeyHash, ItemId>,
    latest_item: Option<ItemId>,
    oldest_in_mem_item: Option<ItemId>,
    oldest_item: Option<ItemId>,
}

impl ActiveState {
    pub fn new() -> Self {
        ActiveState {
            items: Vec::new(),
            used_slots_cnt: AtomicU32::new(0),
            existing_items: DashMap::new(),
            latest_item: None,
            oldest_in_mem_item: None,
            oldest_item: None,
        }
    }

    // Do we need to distinguish between evict vs adding new element? different usecase?
    pub fn batch_put_value_set(
        &mut self,
        value_set: Vec<(StateKey, StateValue)>,
    ) -> Result<()> {
        // TODO(bowu): we should return the updates that need to be persisted
        for (key, value) in value_set {
            let state_key_hash = key.hash();
            // if key in the cache, update the cache only
            if self.items.contains_key(&state_key_hash) {
                let mut item = self.items.get_mut(&state_key_hash).unwrap();
                // update the negihbors' next and previous item
                if let Some(previous_item) = item.previous {
                    previous_item.next = item.next;
                }
                // update the item's next and previous item
                item.next = None;
                item.previous = self.latest_item_key;
                // make the item the latest item
                self.latest_item_key = Some(state_key_hash);
            } else {
                // if the key is not in the cache, check if eviction is required and then try to add the new
                if self.used_slots_cnt >= MAX_ITEMS {
                    // evict the oldest item
                    self.evict_oldest_item()?;
                }
                // find an avaliable slot


            }
        }

    }

    fn evict_oldest_item(&mut self) -> Result<()> {
        // evict the oldest item
        if let Some(oldest_item_key) = self.oldest_item_key {
            let oldest_item = self.items.remove(&oldest_item_key).unwrap();
            self.used_slots_cnt -= 1;
            // update the neighbors
            if let Some(next_item) = oldest_item.next {
                next_item.previous = None;
                self.oldest_item_key = next_item.next;
            } else {
                // no other items in the cache
                self.oldest_item_key = None;
            }
        }
        Ok(())
    }

    fn add_leaf_node(&mut self, key: StateKey, value: StateValue, version: Version) -> Result<()> {
        let state_key_hash = key.hash();
        if self.items.contains_key(&state_key_hash) {
            let mut leaf_node = self.items.get_mut(&state_key_hash).unwrap_or_else(|| {
                panic!("active state tree leaf node not found {}", state_key_hash)
            });
            // skip if the newer updates are already recorded
            if leaf_node.id.version > version {
                return Ok(());
            }

            // Update the value
            if leaf_node.last_used.load(Ordering::SeqCst)
                < self.oldest_usage_count_in_mem_value.load(Ordering::SeqCst)
            {
                // move the value to memory
                leaf_node.value = Value::InMemory {
                    bytes: value.bytes().clone(),
                };

                //TODO(bowu): update the oldest in-mem timestamp in a separate thread
            } else {
                leaf_node.value = Value::InMemory {
                    bytes: value.bytes().clone(),
                };
            }
            // update the timestamp
            leaf_node.last_used.store(
                self.global_usage_count.fetch_add(1, Ordering::SeqCst),
                Ordering::SeqCst,
            );
            return Ok(());
        }
        if self.used_slots_cnt.load(Ordering::SeqCst) >= self.max_occupied_slots {
            self.evict_oldest_leaf_node()?;
        }

        // Add new leaf to the tree

        // If tree is full, we revert the newly added leaf node

        Ok(())
    }

    fn evict_oldest_leaf_node(&mut self) -> Result<LeafNodeId> {
        unimplemented!()
    }

    // reset the gloabl usage count
    // evict old leaf nodes in backgroup jobs
    fn refresh_cache(&mut self) -> Result<()> {
        unimplemented!()
    }

    pub fn get_with_proof(&self, key: HashValue, version: Version) -> Result<ActiveStateTreeProof> {
        unimplemented!()
    }

    pub fn get_with_proof_ext(
        &self,
        key: HashValue,
        version: Version,
    ) -> Result<ActiveStateTreeProof> {
        unimplemented!()
    }

    pub fn get_range_proof(
        &self,
        rightmost_key_to_prove: HashValue,
        version: Version,
    ) -> Result<ActiveStateTreeRangeProof> {
        unimplemented!()
    }
}

// find the next in-mem usage count
pub struct TimestampUpdate {}

pub enum ActiveStateTreeUpdate {
    TimestampUpdate(TimestampUpdate),
    ResetGlobalUsageCount(u64),
    PersistTreeUpdates(TreeDbUpdates),
}
struct ActiveStateTreeMaintainer {
    active_state_tree: Arc<ActiveStateTree>,
    updates_receiver: Receiver<ActiveStateTreeUpdate>,
}
