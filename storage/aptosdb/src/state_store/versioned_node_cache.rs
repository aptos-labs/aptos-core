// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::Node;
use crate::{
    metrics::{NODE_CACHE_HIT, NODE_CACHE_TOTAL},
    OTHER_TIMERS_SECONDS,
};
use aptos_infallible::RwLock;
use aptos_jellyfish_merkle::node_type::NodeKey;
use aptos_types::transaction::Version;
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
    const NUM_VERSIONS_TO_CACHE: usize = 5;

    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Default::default()),
        }
    }

    pub fn add_version(&self, version: Version, nodes: NodeCache) -> Option<Arc<NodeCache>> {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["node_cache_add_version"])
            .start_timer();

        let mut locked = self.inner.write();
        let mut evicted = None;
        if !locked.is_empty() {
            let (last_version, _) = locked.back().unwrap();
            assert!(
                *last_version < version,
                "Updating older version. {} vs latest:{} ",
                version,
                *last_version,
            );

            if locked.len() >= Self::NUM_VERSIONS_TO_CACHE {
                evicted = locked.pop_front().map(|(_ver, cache)| cache);
            }
        }

        locked.push_back((version, Arc::new(nodes)));
        evicted
    }

    pub fn get_version(&self, version: Version) -> Option<Arc<NodeCache>> {
        NODE_CACHE_TOTAL.with_label_values(&["versioned"]).inc();
        self.inner
            .read()
            .iter()
            .rev()
            .find(|(ver, _nodes)| *ver == version)
            .map(|(_ver, nodes)| {
                NODE_CACHE_HIT.with_label_values(&["versioned"]).inc();
                nodes.clone()
            })
    }
}
