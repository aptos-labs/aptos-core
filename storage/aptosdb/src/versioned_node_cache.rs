// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{lru_node_cache::LruNodeCache, state_merkle_db::Node};
use aptos_infallible::RwLock;
use aptos_jellyfish_merkle::node_type::NodeKey;
use aptos_types::transaction::Version;
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
    #[allow(dead_code)]
    pub(crate) const NUM_VERSIONS_TO_CACHE: usize = 2;

    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Default::default()),
        }
    }

    pub fn add_version(&self, _version: Version, _nodes: NodeCache) {
        unimplemented!();
    }

    #[allow(dead_code)]
    pub fn maybe_evict_version(&self, _lru_cache: &LruNodeCache) {
        unimplemented!();
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
