// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{lru_node_cache::LruNodeCache, state_merkle_db::Node, OTHER_TIMERS_SECONDS};
use aptos_infallible::RwLock;
use aptos_jellyfish_merkle::node_type::NodeKey;
use aptos_types::transaction::Version;
use rayon::prelude::*;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

type NodeCache = HashMap<NodeKey, Node>;

#[derive(Debug)]
pub(crate) struct VersionedNodeCache {
    inner: RwLock<VecDeque<(Version, Arc<NodeCache>)>>,
}

impl VersionedNodeCache {
    const NUM_VERSIONS_TO_CACHE: usize = 2;

    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Default::default()),
        }
    }

    pub fn add_version(&self, version: Version, nodes: NodeCache) {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["version_cache_add"])
            .start_timer();

        let mut locked = self.inner.write();
        if !locked.is_empty() {
            let (last_version, _) = locked.back().unwrap();
            assert!(
                *last_version < version,
                "Updating older version. {} vs latest:{} ",
                version,
                *last_version,
            );
        }
        locked.push_back((version, Arc::new(nodes)));
    }

    pub fn maybe_evict_version(&self, lru_cache: &LruNodeCache) {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["version_cache_evict"])
            .start_timer();

        let to_evict = {
            let locked = self.inner.read();
            if locked.len() > Self::NUM_VERSIONS_TO_CACHE {
                locked
                    .front()
                    .map(|(version, cache)| (*version, cache.clone()))
            } else {
                None
            }
        };

        if let Some((version, cache)) = to_evict {
            cache
                .iter()
                .collect::<Vec<_>>()
                .into_par_iter()
                .with_min_len(100)
                .for_each(|(node_key, node)| {
                    lru_cache.put(node_key.clone(), node.clone());
                });

            let evicted = self.inner.write().pop_front();
            assert_eq!(evicted, Some((version, cache)));
        }
    }

    pub fn get_version(&self, version: Version) -> Option<Arc<NodeCache>> {
        self.inner
            .read()
            .iter()
            .rev()
            .find(|(ver, _nodes)| *ver == version)
            .map(|(_ver, nodes)| nodes.clone())
    }
}
