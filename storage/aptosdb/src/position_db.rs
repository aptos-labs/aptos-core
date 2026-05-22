// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Sharded RocksDB tier for native-position value storage.
//!
//! 16 shards keyed by `state_key.get_shard_id()` (the leading nibble
//! of the StateKey hash, matching `state_kv_db` and the JMT internal
//! shard convention), plus a separate per-DB metadata DB. The shard
//! DBs hold the per-key CFs (`position_value`,
//! `stale_position_value_index`); the metadata DB holds the
//! `db_metadata` CF (pruner-progress bookkeeping). Same layout main
//! state's `state_kv_db` uses (shards + metadata DB) — no metadata is
//! ever written to a shard DB.
//!
//! Lifecycle metadata (exchange-id allocations, deny-list) lives in
//! the `aptos_experimental::native_position::ExchangeRegistry` Move
//! resource at `@aptos_framework`, not here. There is no
//! `position_metadata` CF.
//!
//! See `PLAN_native_position.md` for design rationale.

#![forbid(unsafe_code)]

use crate::{
    db_options::{gen_position_cfds, gen_position_metadata_cfds},
    sharded_kv_db::ShardedKvDb,
};
use aptos_config::config::RocksdbConfig;
use aptos_logger::info;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{Cache, Env, DB};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::state_store::NUM_STATE_SHARDS;
use rayon::prelude::*;
use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

/// Number of value-DB shards. Mirrors `aptos_types::state_store::NUM_STATE_SHARDS`.
pub const NUM_NATIVE_VALUE_SHARDS: usize = NUM_STATE_SHARDS;

/// Sharded handle for the position value tier. Thin wrapper around
/// [`ShardedKvDb`] (the 16-shards + metadata-DB substrate shared with
/// main state's `state_kv_db`). Adds position-specific schema reads /
/// writes; layout / routing accessors come through `Deref`.
#[derive(Debug)]
pub struct PositionDb {
    inner: ShardedKvDb,
}

impl Deref for PositionDb {
    type Target = ShardedKvDb;

    fn deref(&self) -> &ShardedKvDb {
        &self.inner
    }
}

impl PositionDb {
    /// Open a sharded `position_db` rooted at `path`. Mirrors
    /// `StateKvDb::new` — opens the metadata DB at `<path>/metadata/`
    /// and 16 shard DBs at `<path>/shard_<i>/` with production CF
    /// tuning. `position_db` has no hot/cold split.
    pub fn new(
        path: &Path,
        rocksdb_config: RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
    ) -> Result<Self> {
        let metadata_db_path = path.join("metadata");
        let metadata_db = Arc::new(Self::open_db(
            metadata_db_path.clone(),
            "position_db_metadata",
            &rocksdb_config,
            env,
            block_cache,
            readonly,
            /* is_metadata = */ true,
        )?);
        info!(
            metadata_db_path = %metadata_db_path.display(),
            "Opened position_db metadata db."
        );

        let shards: [Arc<DB>; NUM_NATIVE_VALUE_SHARDS] = (0..NUM_NATIVE_VALUE_SHARDS)
            .into_par_iter()
            .map(|shard_id| {
                let db =
                    Self::open_shard(path, shard_id, &rocksdb_config, env, block_cache, readonly)
                        .unwrap_or_else(|e| {
                            panic!("Failed to open position_db shard {shard_id}: {e:?}.")
                        });
                Arc::new(db)
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("Collected exactly NUM_NATIVE_VALUE_SHARDS shards");

        Ok(Self {
            inner: ShardedKvDb::new(metadata_db, shards),
        })
    }

    /// Test-only: build a `PositionDb` whose 16 shards + metadata slot
    /// all point at one `Arc<DB>` — defeats per-shard parallelism but
    /// avoids opening 17 RocksDB instances per test.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_uniform_for_test(db: Arc<DB>) -> Self {
        let shards: [Arc<DB>; NUM_NATIVE_VALUE_SHARDS] = std::array::from_fn(|_| Arc::clone(&db));
        Self {
            inner: ShardedKvDb::new(Arc::clone(&db), shards),
        }
    }

    fn open_shard<P: AsRef<Path>>(
        db_root_path: P,
        shard_id: usize,
        rocksdb_config: &RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
    ) -> Result<DB> {
        let shard_path = db_root_path.as_ref().join(format!("shard_{shard_id}"));
        let name = format!("position_db_shard_{shard_id}");
        Self::open_db(
            shard_path,
            &name,
            rocksdb_config,
            env,
            block_cache,
            readonly,
            /* is_metadata = */ false,
        )
    }

    fn open_db(
        path: PathBuf,
        name: &str,
        rocksdb_config: &RocksdbConfig,
        env: Option<&Env>,
        block_cache: Option<&Cache>,
        readonly: bool,
        is_metadata: bool,
    ) -> Result<DB> {
        let rocksdb_opts = gen_rocksdb_options(rocksdb_config, env, readonly);
        let cfds = if is_metadata {
            gen_position_metadata_cfds(rocksdb_config, block_cache)
        } else {
            gen_position_cfds(rocksdb_config, block_cache)
        };
        let res = if readonly {
            DB::open_cf_readonly(rocksdb_opts, path.as_path(), name, cfds)
        } else {
            DB::open_cf(rocksdb_opts, path.as_path(), name, cfds)
        };
        res.map_err(|e| AptosDbError::Other(format!("failed to open {name}: {e}")))
    }
}
