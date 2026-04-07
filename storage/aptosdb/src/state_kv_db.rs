// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    db_options::{gen_hot_state_kv_shard_cfds, gen_state_kv_shard_cfds},
    metrics::OTHER_TIMERS_SECONDS,
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        hot_state_value_by_key_hash::{HotStateEntry, HotStateValueByKeyHashSchema},
        state_value_by_key_hash::StateValueByKeyHashSchema,
    },
    utils::{
        truncation_helper::{get_state_kv_commit_progress, truncate_state_kv_db_shards},
        ShardedStateKvSchemaBatch,
    },
};
use aptos_config::config::{RocksdbConfig, StorageDirPaths};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_logger::prelude::info;
use aptos_metrics_core::TimerHelper;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{
    batch::{SchemaBatch, WriteBatch},
    Cache, Env, ReadOptions, DB,
};
use aptos_storage_interface::Result;
use aptos_types::{
    state_store::{
        hot_state::{LRUEntry, THotStateSlot},
        state_key::StateKey,
        state_slot::{StateSlot, StateSlotKind},
        state_value::StateValue,
        NUM_STATE_SHARDS,
    },
    transaction::Version,
};
use dashmap::DashMap;
use rayon::prelude::*;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

fn db_folder_name(is_hot: bool) -> &'static str {
    if is_hot {
        "hot_state_kv_db"
    } else {
        "state_kv_db"
    }
}

fn metadata_db_name(is_hot: bool) -> &'static str {
    if is_hot {
        "hot_state_kv_metadata_db"
    } else {
        "state_kv_metadata_db"
    }
}

pub struct StateKvDb {
    state_kv_metadata_db: Arc<DB>,
    state_kv_db_shards: [Arc<DB>; NUM_STATE_SHARDS],
    is_hot: bool,
}

impl StateKvDb {
    pub(crate) fn is_hot(&self) -> bool {
        self.is_hot
    }

    fn db_tag(&self) -> &'static str {
        if self.is_hot {
            "hot"
        } else {
            "cold"
        }
    }

    pub(crate) fn new(
        db_paths: &StorageDirPaths,
        state_kv_db_config: RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
        is_hot: bool,
        delete_on_restart: bool,
    ) -> Result<Self> {
        Self::open_sharded(
            db_paths,
            state_kv_db_config,
            env,
            block_cache,
            readonly,
            is_hot,
            delete_on_restart,
        )
    }

    pub(crate) fn open_sharded(
        db_paths: &StorageDirPaths,
        state_kv_db_config: RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
        is_hot: bool,
        delete_on_restart: bool,
    ) -> Result<Self> {
        assert!(
            !delete_on_restart || is_hot,
            "Only hot state can be cleared on restart"
        );

        let metadata_db_root_path = if is_hot {
            db_paths.hot_state_kv_db_metadata_root_path()
        } else {
            db_paths.state_kv_db_metadata_root_path()
        };
        let state_kv_metadata_db_path = Self::metadata_db_path(metadata_db_root_path, is_hot);

        let state_kv_metadata_db = Arc::new(Self::open_db(
            state_kv_metadata_db_path.clone(),
            metadata_db_name(is_hot),
            &state_kv_db_config,
            env,
            block_cache,
            readonly,
            is_hot,
            delete_on_restart,
        )?);

        info!(
            state_kv_metadata_db_path = state_kv_metadata_db_path,
            is_hot = is_hot,
            "Opened state kv metadata db!"
        );

        let state_kv_db_shards = (0..NUM_STATE_SHARDS)
            .into_par_iter()
            .map(|shard_id| {
                let shard_root_path = if is_hot {
                    db_paths.hot_state_kv_db_shard_root_path(shard_id)
                } else {
                    db_paths.state_kv_db_shard_root_path(shard_id)
                };
                let db = Self::open_shard(
                    shard_root_path,
                    shard_id,
                    &state_kv_db_config,
                    env,
                    block_cache,
                    readonly,
                    is_hot,
                    delete_on_restart,
                )
                .unwrap_or_else(|e| {
                    let db_type = if is_hot { "hot state kv" } else { "state kv" };
                    panic!("Failed to open {db_type} db shard {shard_id}: {e:?}.")
                });
                Arc::new(db)
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let state_kv_db = Self {
            state_kv_metadata_db,
            state_kv_db_shards,
            is_hot,
        };

        // TODO(HotState): Integrate hot state KV DB with pruner and add truncation support
        // (stale index tracking, etc.) for the hot state DB.
        if !readonly && !delete_on_restart && !is_hot {
            if let Some(overall_kv_commit_progress) = get_state_kv_commit_progress(&state_kv_db)? {
                truncate_state_kv_db_shards(&state_kv_db, overall_kv_commit_progress)?;
            }
        }

        Ok(state_kv_db)
    }

    pub(crate) fn new_sharded_native_batches(&self) -> ShardedStateKvSchemaBatch<'_> {
        std::array::from_fn(|shard_id| self.db_shard(shard_id).new_native_batch())
    }

    pub(crate) fn commit(
        &self,
        version: Version,
        state_kv_metadata_batch: Option<SchemaBatch>,
        sharded_state_kv_batches: ShardedStateKvSchemaBatch,
    ) -> Result<()> {
        let _timer =
            OTHER_TIMERS_SECONDS.timer_with(&[&format!("{}__state_kv_db__commit", self.db_tag())]);
        {
            let _timer = OTHER_TIMERS_SECONDS
                .timer_with(&[&format!("{}__state_kv_db__commit_shards", self.db_tag())]);
            THREAD_MANAGER.get_io_pool().scope(|s| {
                let mut batches = sharded_state_kv_batches.into_iter();
                for shard_id in 0..NUM_STATE_SHARDS {
                    let state_kv_batch = batches
                        .next()
                        .expect("Not sufficient number of sharded state kv batches");
                    s.spawn(move |_| {
                        // TODO(grao): Consider propagating the error instead of panic, if necessary.
                        self.commit_single_shard(version, shard_id, state_kv_batch)
                            .unwrap_or_else(|err| {
                                panic!("Failed to commit shard {shard_id}: {err}.")
                            });
                    });
                }
            });
        }
        if let Some(batch) = state_kv_metadata_batch {
            let _timer = OTHER_TIMERS_SECONDS
                .timer_with(&[&format!("{}__state_kv_db__commit_metadata", self.db_tag())]);
            self.state_kv_metadata_db.write_schemas(batch)?;
        }

        self.write_progress(version)
    }

    pub(crate) fn write_progress(&self, version: Version) -> Result<()> {
        self.state_kv_metadata_db.put::<DbMetadataSchema>(
            &DbMetadataKey::StateKvCommitProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub(crate) fn write_pruner_progress(&self, version: Version) -> Result<()> {
        self.state_kv_metadata_db.put::<DbMetadataSchema>(
            &DbMetadataKey::StateKvPrunerProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub(crate) fn create_checkpoint(
        db_root_path: impl AsRef<Path>,
        cp_root_path: impl AsRef<Path>,
        is_hot: bool,
    ) -> Result<()> {
        // TODO(grao): Support path override here.
        let state_kv_db = Self::open_sharded(
            &StorageDirPaths::from_path(db_root_path),
            RocksdbConfig::default(),
            None,
            None,
            /* readonly = */ false,
            is_hot,
            /* delete_on_restart = */ false,
        )?;
        let cp_state_kv_db_path = cp_root_path.as_ref().join(db_folder_name(is_hot));

        info!(
            is_hot = is_hot,
            "Creating state_kv_db checkpoint at: {cp_state_kv_db_path:?}"
        );

        std::fs::remove_dir_all(&cp_state_kv_db_path).unwrap_or(());
        std::fs::create_dir_all(&cp_state_kv_db_path).unwrap_or(());

        state_kv_db
            .metadata_db()
            .create_checkpoint(Self::metadata_db_path(cp_root_path.as_ref(), is_hot))?;

        for shard_id in 0..NUM_STATE_SHARDS {
            state_kv_db
                .db_shard(shard_id)
                .create_checkpoint(Self::db_shard_path(cp_root_path.as_ref(), shard_id, is_hot))?;
        }

        Ok(())
    }

    pub(crate) fn metadata_db(&self) -> &DB {
        &self.state_kv_metadata_db
    }

    pub(crate) fn metadata_db_arc(&self) -> Arc<DB> {
        Arc::clone(&self.state_kv_metadata_db)
    }

    pub(crate) fn db_shard(&self, shard_id: usize) -> &DB {
        &self.state_kv_db_shards[shard_id]
    }

    pub(crate) fn db_shard_arc(&self, shard_id: usize) -> Arc<DB> {
        Arc::clone(&self.state_kv_db_shards[shard_id])
    }

    pub(crate) fn num_shards(&self) -> usize {
        NUM_STATE_SHARDS
    }

    pub(crate) fn commit_single_shard(
        &self,
        version: Version,
        shard_id: usize,
        mut batch: impl WriteBatch,
    ) -> Result<()> {
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::StateKvShardCommitProgress(shard_id),
            &DbMetadataValue::Version(version),
        )?;
        self.state_kv_db_shards[shard_id].write_schemas(batch)
    }

    fn open_shard<P: AsRef<Path>>(
        db_root_path: P,
        shard_id: usize,
        state_kv_db_config: &RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
        is_hot: bool,
        delete_on_restart: bool,
    ) -> Result<DB> {
        let db_name = if is_hot {
            format!("hot_state_kv_db_shard_{}", shard_id)
        } else {
            format!("state_kv_db_shard_{}", shard_id)
        };
        Self::open_db(
            Self::db_shard_path(db_root_path, shard_id, is_hot),
            &db_name,
            state_kv_db_config,
            env,
            block_cache,
            readonly,
            is_hot,
            delete_on_restart,
        )
    }

    fn open_db(
        path: PathBuf,
        name: &str,
        state_kv_db_config: &RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
        is_hot: bool,
        delete_on_restart: bool,
    ) -> Result<DB> {
        if delete_on_restart {
            assert!(!readonly, "Should not reset DB in read-only mode.");
            info!("delete_on_restart is true. Removing {path:?} entirely.");
            std::fs::remove_dir_all(&path).unwrap_or(());
        }

        let rocksdb_opts = gen_rocksdb_options(state_kv_db_config, env, readonly);
        let cfds = if is_hot {
            gen_hot_state_kv_shard_cfds(state_kv_db_config, block_cache)
        } else {
            gen_state_kv_shard_cfds(state_kv_db_config, block_cache)
        };

        if readonly {
            DB::open_cf_readonly(rocksdb_opts, path, name, cfds)
        } else {
            DB::open_cf(rocksdb_opts, path, name, cfds)
        }
    }

    fn db_shard_path<P: AsRef<Path>>(db_root_path: P, shard_id: usize, is_hot: bool) -> PathBuf {
        let shard_sub_path = format!("shard_{}", shard_id);
        db_root_path
            .as_ref()
            .join(db_folder_name(is_hot))
            .join(Path::new(&shard_sub_path))
    }

    fn metadata_db_path<P: AsRef<Path>>(db_root_path: P, is_hot: bool) -> PathBuf {
        db_root_path
            .as_ref()
            .join(db_folder_name(is_hot))
            .join("metadata")
    }

    pub(crate) fn get_state_value_with_version_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<(Version, StateValue)>> {
        let mut read_opts = ReadOptions::default();

        // We want `None` if the state_key changes in iteration.
        read_opts.set_prefix_same_as_start(true);
        let mut iter = self
            .db_shard(state_key.get_shard_id())
            .iter_with_opts::<StateValueByKeyHashSchema>(read_opts)?;
        iter.seek(&(state_key.hash(), version))?;
        Ok(iter
            .next()
            .transpose()?
            .and_then(|((_, version), value_opt)| value_opt.map(|value| (version, value))))
    }

    /// Returns the latest hot state entry for the given key at or before the
    /// given version. Outer `None` means no entry found; inner `None` means the
    /// key was evicted at that version.
    #[cfg(test)]
    pub(crate) fn get_hot_state_entry_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<(Version, Option<HotStateEntry>)>> {
        let mut read_opts = ReadOptions::default();
        read_opts.set_prefix_same_as_start(true);
        let mut iter = self
            .db_shard(state_key.get_shard_id())
            .iter_with_opts::<HotStateValueByKeyHashSchema>(read_opts)?;
        iter.seek(&(state_key.hash(), version))?;
        Ok(iter
            .next()
            .transpose()?
            .map(|((_, version), entry_opt)| (version, entry_opt)))
    }

    /// Loads all hot state KV entries from the DB, given the most recently committed version.
    ///
    /// All entries in the DB must have `hot_since_version <= committed_version` (crash recovery
    /// truncation is assumed to have already run). For each unique key hash, picks the most
    /// recent entry. Evicted entries are excluded. The returned shards have correctly assembled
    /// LRU doubly-linked list pointers, ordered by `hot_since_version`.
    #[allow(dead_code)]
    pub(crate) fn load_hot_state_kvs(
        &self,
        committed_version: Version,
    ) -> Result<[LoadedHotStateShard; NUM_STATE_SHARDS]> {
        assert!(
            self.is_hot,
            "load_hot_state_kvs can only be called on hot state KV DB"
        );

        let start = Instant::now();

        let shards: [_; NUM_STATE_SHARDS] = (0..NUM_STATE_SHARDS)
            .into_par_iter()
            .map(|shard_id| self.load_shard(shard_id, committed_version))
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .expect("Collected exactly NUM_STATE_SHARDS results");

        let total_items: usize = shards.iter().map(|s| s.num_items).sum();
        let elapsed = start.elapsed();
        info!(
            total_items = total_items,
            duration_ms = elapsed.as_millis() as u64,
            shard_counts = ?shards.iter().map(|s| s.num_items).collect::<Vec<_>>(),
            "Loaded hot state KVs from DB.",
        );

        Ok(shards)
    }

    fn load_shard(
        &self,
        shard_id: usize,
        committed_version: Version,
    ) -> Result<LoadedHotStateShard> {
        let entries = self.scan_shard_entries(shard_id, committed_version)?;
        let loaded = Self::assemble_lru_chain(entries);
        Ok(loaded)
    }

    // TODO(HotState): The current implementation does a full scan per shard. This can be
    // further sped up (e.g. parallel within-shard scan, prefix-seek per key group, or maintaining
    // a separate index), but is left for later since correctness matters more at this stage.
    /// Scans a single shard DB and returns the most recent hot entry per key_hash.
    /// Evicted keys are excluded. The returned entries have uninitialized LRU pointers.
    fn scan_shard_entries(
        &self,
        shard_id: usize,
        committed_version: Version,
    ) -> Result<Vec<(HashValue, Version, StateSlotKind)>> {
        let mut iter = self
            .db_shard(shard_id)
            .iter::<HotStateValueByKeyHashSchema>()?;
        iter.seek_to_first();

        let mut entries = Vec::new();
        let mut current_key_hash: Option<HashValue> = None;
        let mut found_for_current = false;

        for item in iter {
            let ((key_hash, hot_since_version), entry_opt) = item?;

            // After crash recovery truncation, no entry should exist beyond the
            // committed version.
            assert!(
                hot_since_version <= committed_version,
                "Entry {key_hash} has hot_since_version {hot_since_version} > \
                 committed_version {committed_version}; \
                 DB should have been truncated during crash recovery.",
            );

            // New key group?
            if current_key_hash != Some(key_hash) {
                current_key_hash = Some(key_hash);
                found_for_current = false;
            }

            if found_for_current {
                continue;
            }

            // This is the most recent entry for this key_hash.
            found_for_current = true;

            let kind = match entry_opt {
                None => continue, // Evicted — not hot.
                Some(HotStateEntry::Occupied {
                    value,
                    value_version,
                }) => StateSlotKind::HotOccupied {
                    value_version,
                    value,
                    hot_since_version,
                    lru_info: LRUEntry::uninitialized(),
                },
                Some(HotStateEntry::Vacant) => StateSlotKind::HotVacant {
                    hot_since_version,
                    lru_info: LRUEntry::uninitialized(),
                },
            };

            entries.push((key_hash, hot_since_version, kind));
        }

        Ok(entries)
    }

    /// Sorts entries by `(hot_since_version, key_hash)`, assembles LRU doubly-linked list
    /// pointers, and builds the `DashMap`. Validates the chain before returning.
    fn assemble_lru_chain(
        mut entries: Vec<(HashValue, Version, StateSlotKind)>,
    ) -> LoadedHotStateShard {
        // Sort by (hot_since_version, key_hash) ascending.
        // Index 0 = oldest (LRU tail), last = newest (MRU head).
        entries.sort_by(|a, b| (a.1, a.0).cmp(&(b.1, b.0)));

        let num_items = entries.len();
        let map = DashMap::with_capacity(num_items);

        // Collect key_hashes for neighbor lookups before consuming entries.
        let key_hashes: Vec<_> = entries.iter().map(|(kh, _, _)| *kh).collect();

        for (i, (key_hash, _hot_since_version, kind)) in entries.into_iter().enumerate() {
            let prev = if i + 1 < num_items {
                Some(key_hashes[i + 1])
            } else {
                None
            };
            let next = if i > 0 { Some(key_hashes[i - 1]) } else { None };
            let slot =
                StateSlot::new_without_state_key(kind.with_lru_info(LRUEntry { prev, next }));
            map.insert(key_hash, slot);
        }

        let head = key_hashes.last().copied();
        let tail = key_hashes.first().copied();

        let loaded = LoadedHotStateShard {
            map,
            head,
            tail,
            num_items,
        };
        loaded.validate_lru_chain();
        loaded
    }
}

/// Per-shard data recovered from the hot state KV DB.
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct LoadedHotStateShard {
    /// All hot entries keyed by state key hash.
    pub map: DashMap<HashValue, StateSlot>,
    /// The newest (MRU) entry's key hash. `None` if the shard is empty.
    pub head: Option<HashValue>,
    /// The oldest (LRU) entry's key hash. `None` if the shard is empty.
    pub tail: Option<HashValue>,
    /// Total number of items in this shard.
    pub num_items: usize,
}

impl LoadedHotStateShard {
    /// Validates the LRU doubly-linked list by traversing in both directions and
    /// checking bidirectional pointer consistency.
    pub fn validate_lru_chain(&self) {
        if self.num_items == 0 {
            assert!(self.head.is_none(), "empty shard must have head=None");
            assert!(self.tail.is_none(), "empty shard must have tail=None");
            assert!(self.map.is_empty(), "empty shard must have empty map");
            return;
        }

        assert!(self.head.is_some(), "non-empty shard must have head");
        assert!(self.tail.is_some(), "non-empty shard must have tail");
        assert_eq!(self.map.len(), self.num_items, "map.len() != num_items");

        // Traverse head → tail (following `next` pointers), verifying prev backlinks.
        let mut count = 0;
        let mut prev_key: Option<HashValue> = None;
        let mut current = self.head;
        while let Some(key_hash) = current {
            let slot = self
                .map
                .get(&key_hash)
                .unwrap_or_else(|| panic!("LRU chain: key {key_hash} not found in map"));
            assert!(slot.is_hot(), "LRU chain: entry {key_hash} is not hot");
            assert_eq!(
                slot.prev().copied(),
                prev_key,
                "prev pointer mismatch at {key_hash}"
            );
            prev_key = Some(key_hash);
            current = slot.next().copied();
            count += 1;
        }
        assert_eq!(
            count, self.num_items,
            "LRU chain head→tail traversal visited {count} entries, expected {}",
            self.num_items,
        );

        // Traverse tail → head (following `prev` pointers), verifying next backlinks.
        count = 0;
        let mut next_key: Option<HashValue> = None;
        current = self.tail;
        while let Some(key_hash) = current {
            let slot = self
                .map
                .get(&key_hash)
                .unwrap_or_else(|| panic!("LRU chain (reverse): key {key_hash} not found in map"));
            assert_eq!(
                slot.next().copied(),
                next_key,
                "next pointer mismatch at {key_hash}"
            );
            next_key = Some(key_hash);
            current = slot.prev().copied();
            count += 1;
        }
        assert_eq!(
            count, self.num_items,
            "LRU chain tail→head traversal visited {count} entries, expected {}",
            self.num_items,
        );
    }
}
