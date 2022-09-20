// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::OTHER_TIMERS_SECONDS;
use anyhow::Result;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_infallible::Mutex;
use aptos_jellyfish_merkle::{
    restore::JellyfishMerkleRestore,
    IO_POOL, {Key, TreeReader, TreeWriter, Value},
};
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::{proof::SparseMerkleRangeProof, transaction::Version};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use storage_interface::StateSnapshotReceiver;

#[cfg(test)]
mod restore_test;

/// Key-Value batch that will be written into db atomically with other batches.
pub type StateValueBatch<K, V> = HashMap<(K, Version), V>;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct StateSnapshotProgress {
    pub key_hash: HashValue,
    pub usage: StateStorageUsage,
}

impl StateSnapshotProgress {
    pub fn new(key_hash: HashValue, usage: StateStorageUsage) -> Self {
        Self { key_hash, usage }
    }
}

pub trait StateValueWriter<K, V>: Send + Sync {
    /// Writes a kv batch into storage.
    fn write_kv_batch(
        &self,
        version: Version,
        kv_batch: &StateValueBatch<K, Option<V>>,
        progress: StateSnapshotProgress,
    ) -> Result<()>;

    fn write_usage(&self, version: Version, usage: StateStorageUsage) -> Result<()>;

    fn get_progress(&self, version: Version) -> Result<Option<StateSnapshotProgress>>;
}

struct StateValueRestore<K, V> {
    version: Version,
    db: Arc<dyn StateValueWriter<K, V>>,
}

impl<K: Key + CryptoHash + Eq + Hash, V: Value> StateValueRestore<K, V> {
    pub fn new<D: 'static + StateValueWriter<K, V>>(db: Arc<D>, version: Version) -> Self {
        Self { version, db }
    }

    pub fn add_chunk(&mut self, mut chunk: Vec<(K, V)>) -> Result<()> {
        // load progress
        let progress_opt = self.db.get_progress(self.version)?;

        // skip overlaps
        if let Some(progress) = progress_opt {
            let idx = chunk
                .iter()
                .position(|(k, _v)| CryptoHash::hash(k) > progress.key_hash)
                .unwrap_or(chunk.len());
            chunk = chunk.split_off(idx);
        }

        // quit if all skipped
        if chunk.is_empty() {
            return Ok(());
        }

        // save
        let mut usage = progress_opt.map_or(StateStorageUsage::zero(), |p| p.usage);
        let (last_key, _last_value) = chunk.last().unwrap();
        let last_key_hash = CryptoHash::hash(last_key);
        let kv_batch = chunk
            .into_iter()
            .map(|(k, v)| {
                usage.add_item(k.key_size() + v.value_size());
                ((k, self.version), Some(v))
            })
            .collect();

        self.db.write_kv_batch(
            self.version,
            &kv_batch,
            StateSnapshotProgress::new(last_key_hash, usage),
        )
    }

    pub fn finish(self) -> Result<()> {
        let progress = self.db.get_progress(self.version)?;
        self.db.write_usage(
            self.version,
            progress.map_or(StateStorageUsage::zero(), |p| p.usage),
        )
    }

    pub fn previous_key_hash(&self) -> Result<Option<HashValue>> {
        Ok(self
            .db
            .get_progress(self.version)?
            .map(|progress| progress.key_hash))
    }
}

pub struct StateSnapshotRestore<K, V> {
    tree_restore: Arc<Mutex<Option<JellyfishMerkleRestore<K>>>>,
    kv_restore: Arc<Mutex<Option<StateValueRestore<K, V>>>>,
}

impl<K: Key + CryptoHash + Hash + Eq, V: Value> StateSnapshotRestore<K, V> {
    pub fn new<T: 'static + TreeReader<K> + TreeWriter<K>, S: 'static + StateValueWriter<K, V>>(
        tree_store: &Arc<T>,
        value_store: &Arc<S>,
        version: Version,
        expected_root_hash: HashValue,
        async_commit: bool,
    ) -> Result<Self> {
        Ok(Self {
            tree_restore: Arc::new(Mutex::new(Some(JellyfishMerkleRestore::new(
                Arc::clone(tree_store),
                version,
                expected_root_hash,
                async_commit,
            )?))),
            kv_restore: Arc::new(Mutex::new(Some(StateValueRestore::new(
                Arc::clone(value_store),
                version,
            )))),
        })
    }

    pub fn new_overwrite<T: 'static + TreeWriter<K>, S: 'static + StateValueWriter<K, V>>(
        tree_store: &Arc<T>,
        value_store: &Arc<S>,
        version: Version,
        expected_root_hash: HashValue,
    ) -> Result<Self> {
        Ok(Self {
            tree_restore: Arc::new(Mutex::new(Some(JellyfishMerkleRestore::new_overwrite(
                Arc::clone(tree_store),
                version,
                expected_root_hash,
            )?))),
            kv_restore: Arc::new(Mutex::new(Some(StateValueRestore::new(
                Arc::clone(value_store),
                version,
            )))),
        })
    }

    pub fn previous_key_hash(&self) -> Result<Option<HashValue>> {
        let hash_opt = match (
            self.kv_restore
                .lock()
                .as_ref()
                .unwrap()
                .previous_key_hash()?,
            self.tree_restore
                .lock()
                .as_ref()
                .unwrap()
                .previous_key_hash(),
        ) {
            (None, hash_opt) => hash_opt,
            (hash_opt, None) => hash_opt,
            (Some(hash1), Some(hash2)) => Some(std::cmp::min(hash1, hash2)),
        };
        Ok(hash_opt)
    }

    pub fn wait_for_async_commit(&self) -> Result<()> {
        self.tree_restore
            .lock()
            .as_ref()
            .unwrap()
            .wait_for_async_commit()
    }
}

impl<K: Key + CryptoHash + Hash + Eq, V: Value> StateSnapshotReceiver<K, V>
    for StateSnapshotRestore<K, V>
{
    fn add_chunk(&mut self, chunk: Vec<(K, V)>, proof: SparseMerkleRangeProof) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["state_snapshot_add_chunk"])
            .start_timer();
        // Write KV out first because we are likely to resume according to the rightmost key in the
        // tree after crashing.
        let (r1, r2) = IO_POOL.join(
            || {
                let _timer = OTHER_TIMERS_SECONDS
                    .with_label_values(&["state_value_add_chunk"])
                    .start_timer();
                self.kv_restore
                    .lock()
                    .as_mut()
                    .unwrap()
                    .add_chunk(chunk.clone())
            },
            || {
                let _timer = OTHER_TIMERS_SECONDS
                    .with_label_values(&["jmt_add_chunk"])
                    .start_timer();
                self.tree_restore
                    .lock()
                    .as_mut()
                    .unwrap()
                    .add_chunk_impl(chunk.iter().map(|(k, v)| (k, v.hash())).collect(), proof)
            },
        );
        r1?;
        r2?;
        Ok(())
    }

    fn finish(self) -> Result<()> {
        self.kv_restore.lock().take().unwrap().finish()?;
        self.tree_restore.lock().take().unwrap().finish_impl()
    }

    fn finish_box(self: Box<Self>) -> Result<()> {
        self.kv_restore.lock().take().unwrap().finish()?;
        self.tree_restore.lock().take().unwrap().finish_impl()
    }
}
