// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Groups the native-position pruner managers, the analog of main
//! state's `StatePruner`: the value pruner plus the regular and
//! epoch-snapshot merkle pruners. The value pruner's target is driven
//! from `commit_native_position`; the merkle pruners' from the position
//! merkle batch committer.

#![forbid(unsafe_code)]

use crate::{
    position_db::PositionDb,
    position_merkle_db::PositionMerkleDb,
    pruner::{
        PositionEpochSnapshotPrunerManager, PositionStateMerklePrunerManager,
        PositionValuePrunerManager,
    },
};
use aptos_config::config::{LedgerPrunerConfig, StateMerklePrunerConfig};
use std::sync::Arc;

pub(crate) struct PositionPruner {
    pub(crate) value_pruner: PositionValuePrunerManager,
    pub(crate) state_merkle_pruner: PositionStateMerklePrunerManager,
    pub(crate) epoch_snapshot_pruner: PositionEpochSnapshotPrunerManager,
}

impl PositionPruner {
    pub(crate) fn new(
        kv_db: Arc<PositionDb>,
        merkle_db: Arc<PositionMerkleDb>,
        value_pruner_config: LedgerPrunerConfig,
        state_merkle_pruner_config: StateMerklePrunerConfig,
        epoch_snapshot_pruner_config: StateMerklePrunerConfig,
    ) -> Self {
        Self {
            value_pruner: PositionValuePrunerManager::new(kv_db, value_pruner_config),
            state_merkle_pruner: PositionStateMerklePrunerManager::new(
                Arc::clone(&merkle_db),
                state_merkle_pruner_config,
            ),
            epoch_snapshot_pruner: PositionEpochSnapshotPrunerManager::new(
                merkle_db,
                epoch_snapshot_pruner_config,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{pruner::PrunerManager, schema::stale_node_index::StaleNodeIndexSchema};
    use aptos_config::config::{RocksdbConfig, StorageDirPaths};
    use aptos_crypto::{hash::CryptoHash, HashValue};
    use aptos_schemadb::DB;
    use aptos_temppath::TempPath;
    use aptos_types::{state_store::state_key::StateKey, transaction::Version};

    fn open_dbs() -> (TempPath, Arc<PositionDb>, Arc<PositionMerkleDb>) {
        let tmpdir = TempPath::new();
        std::fs::create_dir_all(tmpdir.path()).unwrap();
        let db_paths = StorageDirPaths::from_path(tmpdir.path());
        let kv_db = Arc::new(
            PositionDb::new(&db_paths, RocksdbConfig::default(), None, None, false)
                .expect("PositionDb::new"),
        );
        let merkle_db = Arc::new(
            PositionMerkleDb::new(&db_paths, RocksdbConfig::default(), None, None, false, 0)
                .expect("PositionMerkleDb::new"),
        );
        (tmpdir, kv_db, merkle_db)
    }

    /// Commit a single-key merkle snapshot, overwriting the prior value
    /// so the superseded nodes go stale.
    fn commit_key(merkle_db: &PositionMerkleDb, version: Version, base: Option<Version>) {
        let key = StateKey::raw(b"position-pruner-test");
        let entry = (HashValue::random(), key.clone());
        let (top_levels_batch, batches_for_shards, _root) = merkle_db
            .merklize_value_set(vec![(key.hash(), Some(&entry))], version, base, None)
            .expect("merklize");
        merkle_db
            .commit(version, top_levels_batch, batches_for_shards)
            .expect("commit");
    }

    fn stale_rows_up_to(merkle_db: &PositionMerkleDb, target: Version) -> usize {
        let count_in = |db: &DB| {
            let mut iter = db.iter::<StaleNodeIndexSchema>().unwrap();
            iter.seek_to_first();
            iter.filter_map(|r| r.ok())
                .filter(|(idx, _)| idx.stale_since_version <= target)
                .count()
        };
        let mut total = count_in(merkle_db.metadata_db());
        for shard_id in 0..merkle_db.num_shards() {
            total += count_in(merkle_db.db_shard(shard_id));
        }
        total
    }

    #[test]
    fn merkle_pruner_in_group_prunes_stale_nodes() {
        let (_tmp, kv_db, merkle_db) = open_dbs();
        commit_key(&merkle_db, 0, None);
        commit_key(&merkle_db, 1, Some(0));
        assert!(
            stale_rows_up_to(&merkle_db, 1) > 0,
            "overwrite leaves stale nodes"
        );

        let pruner = PositionPruner::new(
            kv_db,
            Arc::clone(&merkle_db),
            LedgerPrunerConfig {
                enable: true,
                prune_window: 0,
                batch_size: 1,
                user_pruning_window_offset: 0,
            },
            StateMerklePrunerConfig {
                enable: true,
                prune_window: 0,
                batch_size: 1000,
            },
            StateMerklePrunerConfig {
                enable: true,
                prune_window: 0,
                batch_size: 1000,
            },
        );
        pruner.state_merkle_pruner.wake_and_wait_pruner(1).unwrap();

        assert_eq!(stale_rows_up_to(&merkle_db, 1), 0, "stale nodes pruned");
    }
}
