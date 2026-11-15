// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! On-disk layout shared by every sharded DB (main-state and hot-state kv
//! and merkle, and the native stores): a metadata DB plus
//! `NUM_STATE_SHARDS` shard DBs under `<root>/<folder>/{metadata,shard_i}`.

#![forbid(unsafe_code)]

use aptos_config::config::RocksdbConfig;
use aptos_logger::info;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{Cache, ColumnFamilyDescriptor, Env, DB};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::state_store::NUM_STATE_SHARDS;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// Names of one sharded DB.
pub(crate) struct ShardedDbNames {
    /// Folder under each root path, e.g. `state_kv_db`.
    pub folder: &'static str,
    /// RocksDB name of the metadata DB.
    pub metadata_db: &'static str,
    /// RocksDB name prefix of the shard DBs, completed with `_{shard_id}`.
    pub shard_db_prefix: &'static str,
}

pub(crate) struct ShardedDbOpenOptions<'a> {
    pub rocksdb_config: &'a RocksdbConfig,
    pub env: Option<&'a Env>,
    pub block_cache: Option<&'a Cache>,
    pub readonly: bool,
    /// Removes each DB directory before opening it.
    pub delete_on_restart: bool,
}

impl ShardedDbNames {
    /// Opens the metadata DB under `metadata_root` and each shard under
    /// `shard_root(shard_id)` concurrently. Shards are returned in shard-id
    /// order.
    pub(crate) fn open<'p>(
        &self,
        metadata_root: &Path,
        shard_root: impl Fn(usize) -> &'p Path + Sync,
        options: &ShardedDbOpenOptions,
        gen_cfds: impl Fn(&RocksdbConfig, Option<&Cache>) -> Vec<ColumnFamilyDescriptor> + Sync,
    ) -> Result<(Arc<DB>, [Arc<DB>; NUM_STATE_SHARDS])> {
        let (metadata_db, shards) = std::thread::scope(|s| {
            let metadata_handle = s.spawn(|| {
                open_db(
                    metadata_db_path(metadata_root, self.folder),
                    self.metadata_db,
                    options,
                    &gen_cfds,
                )
            });

            let shard_handles: Vec<_> = (0..NUM_STATE_SHARDS)
                .map(|shard_id| {
                    let (shard_root, gen_cfds) = (&shard_root, &gen_cfds);
                    s.spawn(move || {
                        open_db(
                            db_shard_path(shard_root(shard_id), self.folder, shard_id),
                            &format!("{}_{shard_id}", self.shard_db_prefix),
                            options,
                            gen_cfds,
                        )
                    })
                })
                .collect();

            // Joined in shard-id order so each array index matches its shard id.
            let shards = shard_handles
                .into_iter()
                .map(|handle| handle.join().expect("Shard open thread panicked"))
                .collect::<Result<Vec<_>>>();
            let metadata_db = metadata_handle
                .join()
                .expect("Metadata open thread panicked");
            (metadata_db, shards)
        });

        let shards: [Arc<DB>; NUM_STATE_SHARDS] = shards?
            .try_into()
            .expect("Collected exactly NUM_STATE_SHARDS shards");
        Ok((metadata_db?, shards))
    }

    /// Checkpoints an open sharded DB into `<cp_root>/<folder>`, replacing
    /// any previous checkpoint there.
    pub(crate) fn create_checkpoint(
        &self,
        metadata_db: &DB,
        shards: &[Arc<DB>; NUM_STATE_SHARDS],
        cp_root: &Path,
    ) -> Result<()> {
        let cp = cp_root.join(self.folder);
        info!("Creating {} checkpoint at: {cp:?}", self.folder);
        std::fs::remove_dir_all(&cp).unwrap_or(());
        std::fs::create_dir_all(&cp)
            .map_err(|e| AptosDbError::Other(format!("create_checkpoint mkdir {cp:?}: {e}")))?;
        metadata_db.create_checkpoint(metadata_db_path(cp_root, self.folder))?;
        for (shard_id, shard) in shards.iter().enumerate() {
            shard.create_checkpoint(db_shard_path(cp_root, self.folder, shard_id))?;
        }
        Ok(())
    }
}

fn open_db(
    path: PathBuf,
    name: &str,
    options: &ShardedDbOpenOptions,
    gen_cfds: impl Fn(&RocksdbConfig, Option<&Cache>) -> Vec<ColumnFamilyDescriptor>,
) -> Result<Arc<DB>> {
    if options.delete_on_restart {
        if options.readonly {
            return Err(AptosDbError::Other(
                "Should not reset DB in read-only mode.".into(),
            ));
        }
        info!("delete_on_restart is true. Removing {path:?} entirely.");
        std::fs::remove_dir_all(&path).unwrap_or(());
    }

    let rocksdb_opts = gen_rocksdb_options(options.rocksdb_config, options.env, options.readonly);
    let cfds = gen_cfds(options.rocksdb_config, options.block_cache);
    let res = if options.readonly {
        DB::open_cf_readonly(rocksdb_opts, path.as_path(), name, cfds)
    } else {
        DB::open_cf(rocksdb_opts, path.as_path(), name, cfds)
    };
    res.map(Arc::new)
        .map_err(|e| AptosDbError::Other(format!("Failed to open {name}: {e}")))
}

fn metadata_db_path(root: &Path, folder: &str) -> PathBuf {
    root.join(folder).join("metadata")
}

fn db_shard_path(root: &Path, folder: &str, shard_id: usize) -> PathBuf {
    root.join(folder).join(format!("shard_{shard_id}"))
}
