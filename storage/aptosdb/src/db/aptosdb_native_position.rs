// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::AptosDB,
    native_state_committer::NativeStateCommitter,
    position_buffered_state::{new_empty_position_state, position_state_at_version},
    position_db::{PositionDb, NUM_NATIVE_VALUE_SHARDS},
    position_merkle_db::PositionMerkleDb,
    position_state_store::PositionStateStore,
    utils::truncation_helper::{
        get_position_commit_progress, get_position_merkle_commit_progress,
        truncate_position_db_shards, truncate_position_merkle_db_shards,
    },
};
use aptos_config::config::StorageDirPaths;
use aptos_logger::info;
use aptos_storage_interface::{AptosDbError, Result};
use std::sync::Arc;

/// Flip to `true` once order/collateral land.
pub(crate) const ENABLE_NATIVE_POSITION: bool = false;

pub struct PositionBundle {
    pub kv_db: Arc<PositionDb>,
    pub merkle_db: Arc<PositionMerkleDb>,
    /// `None` in readonly mode.
    pub(crate) state_store: Option<Arc<PositionStateStore>>,
}

impl AptosDB {
    pub fn position(&self) -> Option<&Arc<PositionBundle>> {
        self.position.as_ref()
    }

    pub fn native_state_committer(&self) -> Option<NativeStateCommitter> {
        let bundle = self.position.as_ref()?;
        Some(NativeStateCommitter::new(bundle.kv_db.clone()))
    }

    /// Called automatically from `open_internal` when
    /// `ENABLE_NATIVE_POSITION` is `true`.
    pub fn init_native_position(
        &mut self,
        db_paths: &StorageDirPaths,
        rocksdb_config: aptos_config::config::RocksdbConfig,
        readonly: bool,
    ) -> Result<()> {
        if self.position.is_some() {
            return Err(AptosDbError::Other(
                "init_native_position called twice; native-position subsystem is already \
                 attached to this AptosDB"
                    .to_string(),
            ));
        }

        let env = aptos_schemadb::Env::new()
            .map_err(|e| AptosDbError::Other(format!("failed to create RocksDB env: {e}")))?;

        let position_db = PositionDb::new(db_paths, rocksdb_config, Some(&env), None, readonly)?;
        if !readonly && let Some(progress) = get_position_commit_progress(&position_db)? {
            truncate_position_db_shards(&position_db, progress)?;
        }

        let merkle_db = PositionMerkleDb::new(
            db_paths,
            rocksdb_config,
            Some(&env),
            None,
            readonly,
            /* max_nodes_per_lru_cache_shard */ 0,
        )?;
        let merkle_progress = if readonly {
            None
        } else {
            let progress = get_position_merkle_commit_progress(&merkle_db)?;
            if let Some(p) = progress {
                truncate_position_merkle_db_shards(&merkle_db, p)?;
            }
            progress
        };
        let kv_db = Arc::new(position_db);
        let merkle_db = Arc::new(merkle_db);

        let state_store = if readonly {
            None
        } else {
            let last_snapshot = match merkle_progress {
                Some(version) => {
                    let root_hash = merkle_db.get_root_hash(version)?;
                    position_state_at_version(version, root_hash)
                },
                None => new_empty_position_state(),
            };
            Some(Arc::new(PositionStateStore::new_at_snapshot(
                Arc::clone(&merkle_db),
                Arc::clone(&self.ledger_db),
                last_snapshot,
            )))
        };

        self.position = Some(Arc::new(PositionBundle {
            kv_db,
            merkle_db,
            state_store,
        }));

        info!(
            num_shards = NUM_NATIVE_VALUE_SHARDS,
            readonly = readonly,
            "Native-position subsystem initialized."
        );

        Ok(())
    }
}
