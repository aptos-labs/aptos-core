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
    sharded_kv_db::ShardedKvDb,
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

/// Distinct values of a nibble. Byte 0 of a key hash is the shard in its high nibble plus a low
/// nibble, so each shard owns `NIBBLE_VALUES` consecutive byte-0 values.
const NIBBLE_VALUES: usize = 16;
/// Number of concurrent key-hash sub-ranges scanned per shard when loading hot state on restart.
const NUM_HOT_LOAD_SUBSCANS: usize = 16;
const _: () = assert!(
    NUM_STATE_SHARDS == NIBBLE_VALUES,
    "hot-state sub-range math treats the shard as the high nibble of byte 0",
);
const _: () = assert!(
    NUM_HOT_LOAD_SUBSCANS >= 1
        && NUM_HOT_LOAD_SUBSCANS <= NIBBLE_VALUES
        && NIBBLE_VALUES.is_multiple_of(NUM_HOT_LOAD_SUBSCANS),
    "NUM_HOT_LOAD_SUBSCANS must evenly divide a nibble's values (one of 1, 2, 4, 8, 16)",
);

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
    inner: ShardedKvDb,
    is_hot: bool,
}

impl std::ops::Deref for StateKvDb {
    type Target = ShardedKvDb;

    fn deref(&self) -> &ShardedKvDb {
        &self.inner
    }
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

        // Open the metadata db and all NUM_STATE_SHARDS shards on dedicated threads rather than the
        // rayon pool (sized to the core count), so every db opens concurrently even on hosts with
        // fewer cores. Opening a db is mostly blocking I/O and happens once at startup, so the
        // one-time thread churn is negligible.
        let state_kv_db_config = &state_kv_db_config;
        let (metadata_db, shards) = std::thread::scope(|scope| {
            let metadata_handle = scope.spawn(|| {
                Self::open_db(
                    state_kv_metadata_db_path.clone(),
                    metadata_db_name(is_hot),
                    state_kv_db_config,
                    env,
                    block_cache,
                    readonly,
                    is_hot,
                    delete_on_restart,
                )
            });

            let shard_handles: Vec<_> = (0..NUM_STATE_SHARDS)
                .map(|shard_id| {
                    scope.spawn(move || {
                        let shard_root_path = if is_hot {
                            db_paths.hot_state_kv_db_shard_root_path(shard_id)
                        } else {
                            db_paths.state_kv_db_shard_root_path(shard_id)
                        };
                        let db = Self::open_shard(
                            shard_root_path,
                            shard_id,
                            state_kv_db_config,
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
                })
                .collect();

            // Joined in shard-id order so each array index matches its shard id.
            let shards = shard_handles
                .into_iter()
                .map(|handle| handle.join().expect("State kv shard open thread panicked"))
                .collect::<Vec<_>>();
            let metadata_db = metadata_handle
                .join()
                .expect("State kv metadata open thread panicked");
            (metadata_db, shards)
        });

        let state_kv_metadata_db = Arc::new(metadata_db?);

        info!(
            state_kv_metadata_db_path = state_kv_metadata_db_path,
            is_hot = is_hot,
            "Opened state kv metadata db!"
        );

        let state_kv_db_shards: [_; NUM_STATE_SHARDS] = shards
            .try_into()
            .expect("Collected exactly NUM_STATE_SHARDS shards");

        let state_kv_db = Self {
            inner: ShardedKvDb::new(state_kv_metadata_db, state_kv_db_shards),
            is_hot,
        };

        if !readonly
            && !delete_on_restart
            && let Some(overall_kv_commit_progress) = get_state_kv_commit_progress(&state_kv_db)?
        {
            truncate_state_kv_db_shards(&state_kv_db, overall_kv_commit_progress)?;
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
            self.inner.metadata_db().write_schemas(batch)?;
        }

        self.write_progress(version)
    }

    pub(crate) fn write_progress(&self, version: Version) -> Result<()> {
        self.inner.metadata_db().put::<DbMetadataSchema>(
            &DbMetadataKey::StateKvCommitProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub(crate) fn write_pruner_progress(&self, version: Version) -> Result<()> {
        self.inner.metadata_db().put::<DbMetadataSchema>(
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
        self.inner.metadata_db()
    }

    pub(crate) fn metadata_db_arc(&self) -> Arc<DB> {
        Arc::clone(self.inner.metadata_db())
    }

    pub(crate) fn db_shard(&self, shard_id: usize) -> &DB {
        self.inner.shard(shard_id)
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
        self.inner.shard(shard_id).write_schemas(batch)
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

    /// Returns the latest hot state entry for the given key hash at or before
    /// the given version. Outer `None` means no entry found; inner `None` means
    /// the key was evicted at that version.
    pub(crate) fn get_hot_state_entry_by_version(
        &self,
        key_hash: HashValue,
        version: Version,
    ) -> Result<Option<(Version, Option<HotStateEntry>)>> {
        let mut read_opts = ReadOptions::default();
        read_opts.set_prefix_same_as_start(true);
        let shard_id = usize::from(key_hash.nibble(0));
        let mut iter = self
            .db_shard(shard_id)
            .iter_with_opts::<HotStateValueByKeyHashSchema>(read_opts)?;
        iter.seek(&(key_hash, version))?;
        Ok(iter
            .next()
            .transpose()?
            .map(|((_, version), entry_opt)| (version, entry_opt)))
    }

    /// Loads hot state KV entries from the DB as of `snapshot_version`.
    ///
    /// For each unique key hash, picks the most recent entry with
    /// `hot_since_version <= snapshot_version`. Entries newer than the snapshot version (written
    /// between the snapshot and the committed version) are skipped — they will be replayed during
    /// initialisation. Evicted entries are excluded. The returned shards have correctly assembled
    /// LRU doubly-linked list pointers, ordered by `(hot_since_version, key_hash)`.
    pub(crate) fn load_hot_state_kvs(
        &self,
        snapshot_version: Version,
    ) -> Result<[LoadedHotStateShard; NUM_STATE_SHARDS]> {
        assert!(
            self.is_hot,
            "load_hot_state_kvs can only be called on hot state KV DB"
        );

        let start = Instant::now();

        // Scan each shard's key-hash space in NUM_HOT_LOAD_SUBSCANS contiguous ranges concurrently.
        // The scan is I/O-latency-bound on a cold restart (one random seek per live key), so we run
        // it on dedicated threads rather than the rayon pool (sized to the core count) to let the
        // blocking scans oversubscribe and raise the read queue depth.
        let mut per_shard_entries: [_; NUM_STATE_SHARDS] = std::array::from_fn(|_| Vec::new());
        std::thread::scope(|scope| {
            let mut handles = vec![];
            for shard_id in 0..NUM_STATE_SHARDS {
                for sub in 0..NUM_HOT_LOAD_SUBSCANS {
                    let handle = scope.spawn(move || {
                        self.scan_shard_range(shard_id, sub, snapshot_version)
                            .unwrap_or_else(|e| {
                                panic!(
                                    "Failed to scan hot state shard {shard_id} sub-range {sub} \
                                     at snapshot version {snapshot_version}: {e:?}"
                                )
                            })
                    });
                    handles.push((shard_id, handle));
                }
            }
            for (shard_id, handle) in handles {
                let entries = handle.join().expect("Hot state load thread panicked");
                per_shard_entries[shard_id].extend(entries);
            }
        });

        let shards: [_; NUM_STATE_SHARDS] = per_shard_entries
            .into_par_iter()
            .map(Self::assemble_lru_chain)
            .collect::<Vec<_>>()
            .try_into()
            .expect("Collected exactly NUM_STATE_SHARDS results");

        let total_items: usize = shards.iter().map(|s| s.num_items).sum();
        let elapsed = start.elapsed();
        info!(
            total_items = total_items,
            snapshot_version = snapshot_version,
            duration_ms = elapsed.as_millis() as u64,
            shard_counts = ?shards.iter().map(|s| s.num_items).collect::<Vec<_>>(),
            "Loaded hot state KVs from DB.",
        );

        Ok(shards)
    }

    /// Key-hash bounds `[lo, hi)` of sub-range `sub` within `shard_id`. Byte 0 of a key hash holds
    /// the shard in its high nibble; its 16 low-nibble values are split into NUM_HOT_LOAD_SUBSCANS
    /// equal groups, one per sub-range. `hi == None` (no upper bound) occurs only for the last
    /// sub-range of the last shard, whose upper edge 0x100 overflows byte 0.
    fn shard_subscan_bounds(shard_id: usize, sub: usize) -> (HashValue, Option<HashValue>) {
        let hash_with_byte0 = |byte0: usize| {
            let mut bytes = [0u8; HashValue::LENGTH];
            bytes[0] = byte0 as u8;
            HashValue::new(bytes)
        };

        // `shard_id` is byte 0's high nibble, hence the `* NIBBLE_VALUES`; each sub-range spans
        // `span` low-nibble values, so the next starts `span` further.
        let span = NIBBLE_VALUES / NUM_HOT_LOAD_SUBSCANS;
        let lo_byte0 = shard_id * NIBBLE_VALUES + sub * span;
        let hi_byte0 = lo_byte0 + span;

        let lo = hash_with_byte0(lo_byte0);
        // `hi_byte0 == NUM_STATE_SHARDS * NIBBLE_VALUES` (0x100) overflows byte 0 — no upper bound.
        let hi = (hi_byte0 < NUM_STATE_SHARDS * NIBBLE_VALUES).then(|| hash_with_byte0(hi_byte0));
        (lo, hi)
    }

    fn next_key_hash(key_hash: HashValue) -> Option<HashValue> {
        let mut bytes = *key_hash.as_ref();
        for byte in bytes.iter_mut().rev() {
            if *byte == u8::MAX {
                *byte = 0;
            } else {
                *byte += 1;
                return Some(HashValue::new(bytes));
            }
        }
        None
    }

    /// Scans one key-hash sub-range of a shard DB and returns the most recent hot entry per
    /// key_hash as of `snapshot_version`. Entries newer than the snapshot are skipped. Evicted keys
    /// are excluded. The returned entries have uninitialized LRU pointers.
    fn scan_shard_range(
        &self,
        shard_id: usize,
        sub: usize,
        snapshot_version: Version,
    ) -> Result<Vec<(HashValue, Version, StateSlotKind)>> {
        let (lo, hi) = Self::shard_subscan_bounds(shard_id, sub);

        // Below we seek across key_hash boundaries to skip stale versions of a key. The CF has a
        // prefix bloom filter on key_hash (production config), so without total-order seek the
        // bloom excludes the SST for the absent next prefix and the scan stops after one key.
        let mut read_opts = ReadOptions::default();
        read_opts.set_total_order_seek(true);
        let mut iter = self
            .db_shard(shard_id)
            .iter_with_opts::<HotStateValueByKeyHashSchema>(read_opts)?;
        iter.seek(&(lo, Version::MAX))?;

        let mut entries = Vec::new();

        while let Some(((key_hash, hot_since_version), entry_opt)) = iter.next().transpose()? {
            // Stop once the scan crosses into the next sub-range.
            if hi.is_some_and(|hi| key_hash >= hi) {
                break;
            }

            // Skip entries newer than the snapshot version — they will be replayed.
            if hot_since_version > snapshot_version {
                continue;
            }

            // This is the most recent entry for this key_hash at the snapshot version.
            if let Some(kind) = match entry_opt {
                None => None, // Evicted — not hot.
                Some(HotStateEntry::Occupied {
                    value,
                    value_version,
                }) => Some(StateSlotKind::HotOccupied {
                    value_version,
                    value,
                    hot_since_version,
                    lru_info: LRUEntry::uninitialized(),
                }),
                Some(HotStateEntry::Vacant) => Some(StateSlotKind::HotVacant {
                    hot_since_version,
                    lru_info: LRUEntry::uninitialized(),
                }),
            } {
                entries.push((key_hash, hot_since_version, kind));
            }

            // Older versions for this key_hash sort immediately after this row.
            // Jump to the next key group.
            if let Some(next_key_hash) = Self::next_key_hash(key_hash) {
                iter.seek(&(next_key_hash, Version::MAX))?;
            } else {
                break;
            }
        }

        Ok(entries)
    }

    /// Sorts entries by `(hot_since_version, key_hash)` ascending, assembles the LRU
    /// doubly-linked list, and builds the `DashMap`. Validates the chain before returning.
    ///
    /// That tuple is the canonical LRU order for hot state — runtime insertions into
    /// `HotStateLRU` must follow it, so the chain rebuilt here must match as well.
    fn assemble_lru_chain(
        mut entries: Vec<(HashValue, Version, StateSlotKind)>,
    ) -> LoadedHotStateShard {
        // Index 0 = oldest (LRU tail), last = newest (MRU head).
        entries.sort_by(|a, b| (a.1, a.0).cmp(&(b.1, b.0)));

        let num_items = entries.len();
        let map = DashMap::with_capacity(num_items);

        // Collect key_hashes for neighbor lookups before consuming entries.
        let key_hashes: Vec<_> = entries.iter().map(|(kh, _, _)| *kh).collect();

        let mut total_value_bytes = 0;
        for (i, (key_hash, _hot_since_version, kind)) in entries.into_iter().enumerate() {
            let prev = if i + 1 < num_items {
                Some(key_hashes[i + 1])
            } else {
                None
            };
            let next = if i > 0 { Some(key_hashes[i - 1]) } else { None };
            let slot =
                StateSlot::new_without_state_key(kind.with_lru_info(LRUEntry { prev, next }));
            total_value_bytes += slot.size();
            map.insert(key_hash, slot);
        }

        let head = key_hashes.last().copied();
        let tail = key_hashes.first().copied();

        let loaded = LoadedHotStateShard {
            map,
            head,
            tail,
            num_items,
            total_value_bytes,
        };
        loaded.validate_lru_chain();
        loaded
    }
}

/// Per-shard data recovered from the hot state KV DB.
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
    /// Sum of `StateSlot::size()` across all entries in this shard.
    pub total_value_bytes: usize,
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
