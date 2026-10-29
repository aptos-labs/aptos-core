// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Storage for a trading-native store: a sharded value DB plus a sharded
//! JMT, with its commit pipeline, pruners, truncation and snapshot
//! restore. Each store lives in its own DBs, so all stores share the value,
//! stale-index and JMT column families; a [`NativeStoreSpec`] names the
//! store's folders, metadata keys, pruners and threads. [`PositionStore`]
//! is the native positions.

#![forbid(unsafe_code)]

pub(crate) mod buffered_state;
pub(crate) mod kv_db;
pub(crate) mod merkle_batch_committer;
pub(crate) mod merkle_db;
pub(crate) mod pruner;
pub(crate) mod snapshot_committer;
pub(crate) mod state_store;
pub(crate) mod state_sync;

use crate::{
    common::{
        COLLATERAL_DB_FOLDER_NAME, COLLATERAL_MERKLE_DB_FOLDER_NAME, ORDER_DB_FOLDER_NAME,
        ORDER_MERKLE_DB_FOLDER_NAME, POSITION_DB_FOLDER_NAME, POSITION_MERKLE_DB_FOLDER_NAME,
    },
    pruner::{
        CollateralEpochSnapshot, CollateralStateMerkle, CollateralValue, MerklePrunerSchema,
        OrderEpochSnapshot, OrderStateMerkle, OrderValue, PositionEpochSnapshot,
        PositionStateMerkle, PositionValue, StateValuePrunerSchema,
    },
    schema::{
        db_metadata::DbMetadataKey, native_stale_value_index::NativeStaleValueIndexSchema,
        native_value::NativeValueSchema, stale_node_index::StaleNodeIndexSchema,
        stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
    },
};
use aptos_config::config::StorageDirPaths;
use aptos_types::transaction::Version;
use std::path::PathBuf;

pub(crate) trait NativeStoreSpec: 'static + Send + Sync {
    type ValuePruner: StateValuePrunerSchema<
        ValueSchema = NativeValueSchema,
        StaleIndexSchema = NativeStaleValueIndexSchema,
    >;
    type StateMerklePruner: MerklePrunerSchema<StaleIndexSchema = StaleNodeIndexSchema>;
    type EpochSnapshotPruner: MerklePrunerSchema<StaleIndexSchema = StaleNodeIndexCrossEpochSchema>;

    /// Store name in errors, logs and the JMT metrics tag.
    const NAME: &'static str;
    const KV_DB_FOLDER_NAME: &'static str;
    const MERKLE_DB_FOLDER_NAME: &'static str;
    const KV_METADATA_DB_NAME: &'static str;
    const KV_SHARD_DB_NAME_PREFIX: &'static str;
    const MERKLE_METADATA_DB_NAME: &'static str;
    const MERKLE_SHARD_DB_NAME_PREFIX: &'static str;
    const SNAPSHOT_THREAD_NAME: &'static str;
    const BATCH_THREAD_NAME: &'static str;

    fn kv_db_metadata_root_path(db_paths: &StorageDirPaths) -> &PathBuf;
    fn kv_db_shard_root_path(db_paths: &StorageDirPaths, shard_id: usize) -> &PathBuf;
    fn merkle_db_metadata_root_path(db_paths: &StorageDirPaths) -> &PathBuf;
    fn merkle_db_shard_root_path(db_paths: &StorageDirPaths, shard_id: usize) -> &PathBuf;

    fn commit_progress_key() -> DbMetadataKey;
    fn shard_commit_progress_key(shard_id: usize) -> DbMetadataKey;
    fn snapshot_kv_restore_progress_key(version: Version) -> DbMetadataKey;
}

/// Native positions, keyed by `(exchange, account, market)`.
pub(crate) enum PositionStore {}

/// Native orders, keyed by `(exchange, market, order_id)` and
/// `(exchange, market, account)` for bulk orders.
// Constructed by the order write path (PR 6); until then only tests use it.
#[allow(dead_code)]
pub(crate) enum OrderStore {}

/// Native collateral, keyed by `(exchange, account, scope)`.
// Attached to AptosDB in a later change; until then only tests use it.
#[allow(dead_code)]
pub(crate) enum CollateralStore {}

impl NativeStoreSpec for PositionStore {
    type EpochSnapshotPruner = PositionEpochSnapshot;
    type StateMerklePruner = PositionStateMerkle;
    type ValuePruner = PositionValue;

    const BATCH_THREAD_NAME: &'static str = "position_merkle_batch_committer";
    const KV_DB_FOLDER_NAME: &'static str = POSITION_DB_FOLDER_NAME;
    const KV_METADATA_DB_NAME: &'static str = "position_db_metadata";
    const KV_SHARD_DB_NAME_PREFIX: &'static str = "position_db_shard";
    const MERKLE_DB_FOLDER_NAME: &'static str = POSITION_MERKLE_DB_FOLDER_NAME;
    const MERKLE_METADATA_DB_NAME: &'static str = "position_merkle_db_metadata";
    const MERKLE_SHARD_DB_NAME_PREFIX: &'static str = "position_merkle_db_shard";
    const NAME: &'static str = "position";
    const SNAPSHOT_THREAD_NAME: &'static str = "position_snapshot_committer";

    fn kv_db_metadata_root_path(db_paths: &StorageDirPaths) -> &PathBuf {
        db_paths.position_db_metadata_root_path()
    }

    fn kv_db_shard_root_path(db_paths: &StorageDirPaths, shard_id: usize) -> &PathBuf {
        db_paths.position_db_shard_root_path(shard_id)
    }

    fn merkle_db_metadata_root_path(db_paths: &StorageDirPaths) -> &PathBuf {
        db_paths.position_merkle_db_metadata_root_path()
    }

    fn merkle_db_shard_root_path(db_paths: &StorageDirPaths, shard_id: usize) -> &PathBuf {
        db_paths.position_merkle_db_shard_root_path(shard_id)
    }

    fn commit_progress_key() -> DbMetadataKey {
        DbMetadataKey::PositionCommitProgress
    }

    fn shard_commit_progress_key(shard_id: usize) -> DbMetadataKey {
        DbMetadataKey::PositionShardCommitProgress(shard_id)
    }

    fn snapshot_kv_restore_progress_key(version: Version) -> DbMetadataKey {
        DbMetadataKey::PositionSnapshotKvRestoreProgress(version)
    }
}

impl NativeStoreSpec for OrderStore {
    type EpochSnapshotPruner = OrderEpochSnapshot;
    type StateMerklePruner = OrderStateMerkle;
    type ValuePruner = OrderValue;

    const BATCH_THREAD_NAME: &'static str = "order_merkle_batch_committer";
    const KV_DB_FOLDER_NAME: &'static str = ORDER_DB_FOLDER_NAME;
    const KV_METADATA_DB_NAME: &'static str = "order_db_metadata";
    const KV_SHARD_DB_NAME_PREFIX: &'static str = "order_db_shard";
    const MERKLE_DB_FOLDER_NAME: &'static str = ORDER_MERKLE_DB_FOLDER_NAME;
    const MERKLE_METADATA_DB_NAME: &'static str = "order_merkle_db_metadata";
    const MERKLE_SHARD_DB_NAME_PREFIX: &'static str = "order_merkle_db_shard";
    const NAME: &'static str = "order";
    const SNAPSHOT_THREAD_NAME: &'static str = "order_snapshot_committer";

    fn kv_db_metadata_root_path(db_paths: &StorageDirPaths) -> &PathBuf {
        db_paths.order_db_metadata_root_path()
    }

    fn kv_db_shard_root_path(db_paths: &StorageDirPaths, shard_id: usize) -> &PathBuf {
        db_paths.order_db_shard_root_path(shard_id)
    }

    fn merkle_db_metadata_root_path(db_paths: &StorageDirPaths) -> &PathBuf {
        db_paths.order_merkle_db_metadata_root_path()
    }

    fn merkle_db_shard_root_path(db_paths: &StorageDirPaths, shard_id: usize) -> &PathBuf {
        db_paths.order_merkle_db_shard_root_path(shard_id)
    }

    fn commit_progress_key() -> DbMetadataKey {
        DbMetadataKey::OrderCommitProgress
    }

    fn shard_commit_progress_key(shard_id: usize) -> DbMetadataKey {
        DbMetadataKey::OrderShardCommitProgress(shard_id)
    }

    fn snapshot_kv_restore_progress_key(version: Version) -> DbMetadataKey {
        DbMetadataKey::OrderSnapshotKvRestoreProgress(version)
    }
}

impl NativeStoreSpec for CollateralStore {
    type EpochSnapshotPruner = CollateralEpochSnapshot;
    type StateMerklePruner = CollateralStateMerkle;
    type ValuePruner = CollateralValue;

    const BATCH_THREAD_NAME: &'static str = "collateral_merkle_batch_committer";
    const KV_DB_FOLDER_NAME: &'static str = COLLATERAL_DB_FOLDER_NAME;
    const KV_METADATA_DB_NAME: &'static str = "collateral_db_metadata";
    const KV_SHARD_DB_NAME_PREFIX: &'static str = "collateral_db_shard";
    const MERKLE_DB_FOLDER_NAME: &'static str = COLLATERAL_MERKLE_DB_FOLDER_NAME;
    const MERKLE_METADATA_DB_NAME: &'static str = "collateral_merkle_db_metadata";
    const MERKLE_SHARD_DB_NAME_PREFIX: &'static str = "collateral_merkle_db_shard";
    const NAME: &'static str = "collateral";
    const SNAPSHOT_THREAD_NAME: &'static str = "collateral_snapshot_committer";

    fn kv_db_metadata_root_path(db_paths: &StorageDirPaths) -> &PathBuf {
        db_paths.collateral_db_metadata_root_path()
    }

    fn kv_db_shard_root_path(db_paths: &StorageDirPaths, shard_id: usize) -> &PathBuf {
        db_paths.collateral_db_shard_root_path(shard_id)
    }

    fn merkle_db_metadata_root_path(db_paths: &StorageDirPaths) -> &PathBuf {
        db_paths.collateral_merkle_db_metadata_root_path()
    }

    fn merkle_db_shard_root_path(db_paths: &StorageDirPaths, shard_id: usize) -> &PathBuf {
        db_paths.collateral_merkle_db_shard_root_path(shard_id)
    }

    fn commit_progress_key() -> DbMetadataKey {
        DbMetadataKey::CollateralCommitProgress
    }

    fn shard_commit_progress_key(shard_id: usize) -> DbMetadataKey {
        DbMetadataKey::CollateralShardCommitProgress(shard_id)
    }

    fn snapshot_kv_restore_progress_key(version: Version) -> DbMetadataKey {
        DbMetadataKey::CollateralSnapshotKvRestoreProgress(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::{ORDER_DB_FOLDER_NAME, ORDER_MERKLE_DB_FOLDER_NAME, POSITION_DB_FOLDER_NAME},
        native_store::{kv_db::NativeKvDb, merkle_db::NativeMerkleDb},
        schema::db_metadata::DbMetadataKey,
    };
    use aptos_config::config::{RocksdbConfig, StorageDirPaths};
    use aptos_temppath::TempPath;

    #[test]
    fn order_and_position_stores_have_distinct_names_and_keys() {
        assert_ne!(OrderStore::NAME, PositionStore::NAME);
        assert_ne!(
            OrderStore::KV_DB_FOLDER_NAME,
            PositionStore::KV_DB_FOLDER_NAME
        );
        assert_ne!(
            OrderStore::MERKLE_DB_FOLDER_NAME,
            PositionStore::MERKLE_DB_FOLDER_NAME
        );
        assert_ne!(
            OrderStore::KV_METADATA_DB_NAME,
            PositionStore::KV_METADATA_DB_NAME
        );
        assert_ne!(
            OrderStore::MERKLE_SHARD_DB_NAME_PREFIX,
            PositionStore::MERKLE_SHARD_DB_NAME_PREFIX
        );
        assert_ne!(
            OrderStore::commit_progress_key(),
            PositionStore::commit_progress_key()
        );
        assert_eq!(
            OrderStore::commit_progress_key(),
            DbMetadataKey::OrderCommitProgress
        );
        assert_eq!(
            OrderStore::shard_commit_progress_key(3),
            DbMetadataKey::OrderShardCommitProgress(3)
        );
    }

    #[test]
    fn order_and_position_kv_dbs_open_in_their_own_folders() {
        let tmpdir = TempPath::new();
        tmpdir.create_as_dir().unwrap();
        let db_paths = StorageDirPaths::from_path(tmpdir.path());

        let orders: NativeKvDb<OrderStore> =
            NativeKvDb::new(&db_paths, RocksdbConfig::default(), None, None, false).unwrap();
        let positions: NativeKvDb<PositionStore> =
            NativeKvDb::new(&db_paths, RocksdbConfig::default(), None, None, false).unwrap();
        drop((orders, positions));

        assert!(tmpdir.path().join(ORDER_DB_FOLDER_NAME).is_dir());
        assert!(tmpdir.path().join(POSITION_DB_FOLDER_NAME).is_dir());
        assert!(tmpdir
            .path()
            .join(ORDER_DB_FOLDER_NAME)
            .join("metadata")
            .is_dir());
    }

    #[test]
    fn order_merkle_db_opens_in_its_own_folder() {
        let tmpdir = TempPath::new();
        tmpdir.create_as_dir().unwrap();
        let db_paths = StorageDirPaths::from_path(tmpdir.path());
        let _orders: NativeMerkleDb<OrderStore> =
            NativeMerkleDb::new(&db_paths, RocksdbConfig::default(), None, None, false, 0).unwrap();
        assert!(tmpdir.path().join(ORDER_MERKLE_DB_FOLDER_NAME).is_dir());
    }

    #[test]
    fn collateral_store_names_and_keys_are_distinct() {
        for (name, kv, merkle, metadata, shard_prefix, progress) in [
            (
                PositionStore::NAME,
                PositionStore::KV_DB_FOLDER_NAME,
                PositionStore::MERKLE_DB_FOLDER_NAME,
                PositionStore::KV_METADATA_DB_NAME,
                PositionStore::MERKLE_SHARD_DB_NAME_PREFIX,
                PositionStore::commit_progress_key(),
            ),
            (
                OrderStore::NAME,
                OrderStore::KV_DB_FOLDER_NAME,
                OrderStore::MERKLE_DB_FOLDER_NAME,
                OrderStore::KV_METADATA_DB_NAME,
                OrderStore::MERKLE_SHARD_DB_NAME_PREFIX,
                OrderStore::commit_progress_key(),
            ),
        ] {
            assert_ne!(CollateralStore::NAME, name);
            assert_ne!(CollateralStore::KV_DB_FOLDER_NAME, kv);
            assert_ne!(CollateralStore::MERKLE_DB_FOLDER_NAME, merkle);
            assert_ne!(CollateralStore::KV_METADATA_DB_NAME, metadata);
            assert_ne!(CollateralStore::MERKLE_SHARD_DB_NAME_PREFIX, shard_prefix);
            assert_ne!(CollateralStore::commit_progress_key(), progress);
        }
        assert_eq!(
            CollateralStore::commit_progress_key(),
            DbMetadataKey::CollateralCommitProgress
        );
        assert_eq!(
            CollateralStore::shard_commit_progress_key(3),
            DbMetadataKey::CollateralShardCommitProgress(3)
        );
    }

    #[test]
    fn collateral_dbs_open_in_their_own_folders() {
        let tmpdir = TempPath::new();
        tmpdir.create_as_dir().unwrap();
        let db_paths = StorageDirPaths::from_path(tmpdir.path());

        let kv: NativeKvDb<CollateralStore> =
            NativeKvDb::new(&db_paths, RocksdbConfig::default(), None, None, false).unwrap();
        let merkle: NativeMerkleDb<CollateralStore> =
            NativeMerkleDb::new(&db_paths, RocksdbConfig::default(), None, None, false, 0).unwrap();
        drop((kv, merkle));

        assert!(tmpdir.path().join(COLLATERAL_DB_FOLDER_NAME).is_dir());
        assert!(tmpdir
            .path()
            .join(COLLATERAL_DB_FOLDER_NAME)
            .join("metadata")
            .is_dir());
        assert!(tmpdir
            .path()
            .join(COLLATERAL_MERKLE_DB_FOLDER_NAME)
            .is_dir());
    }
}
