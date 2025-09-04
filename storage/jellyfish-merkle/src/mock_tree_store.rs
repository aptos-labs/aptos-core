// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    node_type::{LeafNode, Node, NodeKey},
    NodeBatch, Result, StaleNodeIndex, TreeReader, TreeUpdateBatch, TreeWriter,
};
use velor_infallible::RwLock;
use velor_storage_interface::{db_ensure as ensure, db_other_bail, VelorDbError};
use velor_types::transaction::Version;
use std::collections::{hash_map::Entry, BTreeSet, HashMap};
pub struct MockTreeStore<K> {
    data: RwLock<(HashMap<NodeKey, Node<K>>, BTreeSet<StaleNodeIndex>)>,
    allow_overwrite: bool,
}

impl<K> Default for MockTreeStore<K> {
    fn default() -> Self {
        Self {
            data: RwLock::new((HashMap::new(), BTreeSet::new())),
            allow_overwrite: false,
        }
    }
}

impl<K> TreeReader<K> for MockTreeStore<K>
where
    K: crate::TestKey,
{
    fn get_node_option(&self, node_key: &NodeKey, _tag: &str) -> Result<Option<Node<K>>> {
        Ok(self.data.read().0.get(node_key).cloned())
    }

    fn get_rightmost_leaf(&self, version: Version) -> Result<Option<(NodeKey, LeafNode<K>)>> {
        let locked = self.data.read();
        let mut node_key_and_node: Option<(NodeKey, LeafNode<K>)> = None;

        for (key, value) in locked.0.iter() {
            if let Node::Leaf(leaf_node) = value {
                if key.version() == version
                    && (node_key_and_node.is_none()
                        || leaf_node.account_key()
                            > node_key_and_node.as_ref().unwrap().1.account_key())
                {
                    node_key_and_node.replace((key.clone(), leaf_node.clone()));
                }
            }
        }

        Ok(node_key_and_node)
    }
}

impl<K> TreeWriter<K> for MockTreeStore<K>
where
    K: crate::TestKey,
{
    fn write_node_batch(&self, node_batch: &NodeBatch<K>) -> Result<()> {
        let mut locked = self.data.write();
        for (node_key, node) in node_batch.clone() {
            let replaced = locked.0.insert(node_key, node);
            if !self.allow_overwrite {
                assert_eq!(replaced, None);
            }
        }
        Ok(())
    }
}

impl<K> MockTreeStore<K>
where
    K: crate::TestKey,
{
    pub fn new(allow_overwrite: bool) -> Self {
        Self {
            allow_overwrite,
            ..Default::default()
        }
    }

    pub fn put_node(&self, node_key: NodeKey, node: Node<K>) -> Result<()> {
        match self.data.write().0.entry(node_key) {
            Entry::Occupied(o) => db_other_bail!("Key {:?} exists.", o.key()),
            Entry::Vacant(v) => {
                v.insert(node);
            },
        }
        Ok(())
    }

    fn put_stale_node_index(&self, index: StaleNodeIndex) -> Result<()> {
        let is_new_entry = self.data.write().1.insert(index);
        ensure!(is_new_entry, "Duplicated retire log.");
        Ok(())
    }

    pub fn write_tree_update_batch(&self, batch: TreeUpdateBatch<K>) -> Result<()> {
        batch
            .node_batch
            .into_iter()
            .flatten()
            .map(|(k, v)| self.put_node(k, v))
            .collect::<Result<Vec<_>>>()?;
        batch
            .stale_node_index_batch
            .into_iter()
            .flatten()
            .map(|i| self.put_stale_node_index(i))
            .collect::<Result<Vec<_>>>()?;
        Ok(())
    }

    pub fn purge_stale_nodes(&self, min_readable_version: Version) -> Result<()> {
        let mut wlocked = self.data.write();

        // Only records retired before or at `min_readable_version` can be purged in order
        // to keep that version still readable.
        let to_prune = wlocked
            .1
            .iter()
            .take_while(|log| log.stale_since_version <= min_readable_version)
            .cloned()
            .collect::<Vec<_>>();

        for log in to_prune {
            let removed = wlocked.0.remove(&log.node_key).is_some();
            ensure!(removed, "Stale node index refers to non-existent node.");
            wlocked.1.remove(&log);
        }

        Ok(())
    }

    pub fn num_nodes(&self) -> usize {
        self.data.read().0.len()
    }
}
