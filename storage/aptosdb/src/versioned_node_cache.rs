// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{lru_node_cache::LruNodeCache, metrics::OTHER_TIMERS_SECONDS, state_merkle_db::Node};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_infallible::RwLock;
use aptos_jellyfish_merkle::node_type::NodeKey;
use aptos_metrics_core::TimerHelper;
use aptos_types::transaction::Version;
use rayon::prelude::*;
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    sync::Arc,
};

type NodeCache = HashMap<NodeKey, Node>;

pub(crate) struct VersionedNodeCache {
    inner: RwLock<VecDeque<(Version, Arc<NodeCache>)>>,
}

impl fmt::Debug for VersionedNodeCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entries = self.inner.read();
        writeln!(f, "Total versions: {}.", entries.len())?;
        for entry in entries.iter() {
            writeln!(f, "Version {} has {} elements.", entry.0, entry.1.len())?;
        }
        Ok(())
    }
}

impl VersionedNodeCache {
    pub(crate) const NUM_VERSIONS_TO_CACHE: usize = 2;

    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Default::default()),
        }
    }

    pub fn add_version(&self, version: Version, nodes: NodeCache) {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["version_cache_add"]);

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
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["version_cache_evict"]);

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
            THREAD_MANAGER.get_non_exe_cpu_pool().install(|| {
                cache
                    .iter()
                    .collect::<Vec<_>>()
                    .into_par_iter()
                    .with_min_len(100)
                    .for_each(|(node_key, node)| {
                        lru_cache.put(node_key.clone(), node.clone());
                    });
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
