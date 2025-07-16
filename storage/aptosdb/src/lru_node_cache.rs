// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_merkle_db::Node;
use aptos_infallible::Mutex;
use aptos_jellyfish_merkle::node_type::NodeKey;
use aptos_types::{nibble::nibble_path::NibblePath, transaction::Version};
use lru::LruCache;
use std::{fmt, num::NonZeroUsize};

const NUM_SHARDS: usize = 256;

pub(crate) struct LruNodeCache {
    shards: [Mutex<LruCache<NibblePath, (Version, Node)>>; NUM_SHARDS],
}

impl fmt::Debug for LruNodeCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "LruCache with {NUM_SHARDS} shards.")
    }
}

impl LruNodeCache {
    pub fn new(max_nodes_per_shard: NonZeroUsize) -> Self {
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
        let mut r = self.shards[Self::shard(node_key.nibble_path()) as usize].lock();
        let ret = r.get(node_key.nibble_path()).and_then(|(version, node)| {
            if *version == node_key.version() {
                Some(node.clone())
            } else {
                None
            }
        });
        ret
    }

    pub fn put(&self, node_key: NodeKey, node: Node) {
        let (version, nibble_path) = node_key.unpack();
        let mut w = self.shards[Self::shard(&nibble_path) as usize].lock();
        let value = (version, node);
        w.put(nibble_path, value);
    }
}
