// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::Node;
use crate::metrics::{NODE_CACHE_HIT, NODE_CACHE_TOTAL};
use aptos_infallible::Mutex;
use aptos_jellyfish_merkle::node_type::NodeKey;
use aptos_types::{nibble::nibble_path::NibblePath, transaction::Version};
use lru::LruCache;

const NUM_SHARDS: usize = 256;

#[derive(Debug)]
pub(crate) struct LruNodeCache {
    shards: [Mutex<LruCache<NibblePath, (Version, Node)>>; NUM_SHARDS],
}

impl LruNodeCache {
    pub fn new(max_nodes_per_shard: usize) -> Self {
        Self {
            // `arr!()` doesn't allow a const in place of the integer literal
            shards: arr_macro::arr![Mutex::new(LruCache::new(max_nodes_per_shard)); 256],
        }
    }

    fn shard(nibble_path: &NibblePath) -> u8 {
        let path_bytes = nibble_path.bytes();
        if path_bytes.is_empty() {
            0
        } else {
            path_bytes[0]
        }
    }

    pub fn get(&self, node_key: &NodeKey) -> Option<Node> {
        NODE_CACHE_TOTAL.with_label_values(&["position_lru"]).inc();
        self.shards[Self::shard(node_key.nibble_path()) as usize]
            .lock()
            .get(node_key.nibble_path())
            .and_then(|(version, node)| {
                if *version == node_key.version() {
                    NODE_CACHE_HIT.with_label_values(&["position_lru"]).inc();
                    Some(node.clone())
                } else {
                    None
                }
            })
    }

    pub fn put(&self, node_key: NodeKey, node: Node) {
        let (version, nibble_path) = node_key.unpack();
        self.shards[Self::shard(&nibble_path) as usize]
            .lock()
            .put(nibble_path, (version, node));
    }
}
