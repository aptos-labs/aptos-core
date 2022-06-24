// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::Node;
use crate::metrics::{NODE_CACHE_HIT, NODE_CACHE_TOTAL};
use aptos_infallible::Mutex;
use aptos_jellyfish_merkle::node_type::NodeKey;
use lru::LruCache;

const NUM_SHARDS: usize = 256;

#[derive(Debug)]
pub(crate) struct LruNodeCache {
    shards: [Mutex<LruCache<NodeKey, Node>>; NUM_SHARDS],
}

impl LruNodeCache {
    pub fn new(max_nodes_per_shard: usize) -> Self {
        Self {
            // `arr!()` doesn't allow a const in place of the integer literal
            shards: arr_macro::arr![Mutex::new(LruCache::new(max_nodes_per_shard)); 256],
        }
    }

    fn shard(node_key: &NodeKey) -> u8 {
        let path_bytes = node_key.nibble_path().bytes();
        if path_bytes.is_empty() {
            0
        } else {
            path_bytes[0]
        }
    }

    pub fn get(&self, node_key: &NodeKey) -> Option<Node> {
        NODE_CACHE_TOTAL.with_label_values(&["lru"]).inc();
        self.shards[Self::shard(node_key) as usize]
            .lock()
            .get(node_key)
            .map(|node| {
                NODE_CACHE_HIT.with_label_values(&["lru"]).inc();
                node.clone()
            })
    }

    pub fn put(&self, node_key: NodeKey, node: Node) -> Option<Node> {
        self.shards[Self::shard(&node_key) as usize]
            .lock()
            .put(node_key, node)
    }
}
