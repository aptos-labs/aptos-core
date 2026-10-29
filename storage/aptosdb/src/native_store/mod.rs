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
pub(crate) mod bundle;
pub(crate) mod kv_db;
pub(crate) mod merkle_batch_committer;
pub(crate) mod merkle_db;
pub(crate) mod pruner;
pub(crate) mod snapshot_committer;
pub(crate) mod state_store;
pub(crate) mod state_sync;

use crate::{
    common::{
        ORDER_DB_FOLDER_NAME, ORDER_MERKLE_DB_FOLDER_NAME, POSITION_DB_FOLDER_NAME,
        POSITION_MERKLE_DB_FOLDER_NAME,
    },
    native_metrics::{ORDER_WRITES, POSITION_WRITES},
    pruner::{
        MerklePrunerSchema, OrderEpochSnapshot, OrderStateMerkle, OrderValue,
        PositionEpochSnapshot, PositionStateMerkle, PositionValue, StateValuePrunerSchema,
    },
    schema::{
        db_metadata::DbMetadataKey, native_stale_value_index::NativeStaleValueIndexSchema,
        native_value::NativeValueSchema, stale_node_index::StaleNodeIndexSchema,
        stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
    },
};
use aptos_config::config::StorageDirPaths;
use aptos_metrics_core::IntCounterVec;
use aptos_types::{
    state_store::state_key::StateKey,
    transaction::Version,
    write_set::{WriteOp, WriteSet},
};
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

    /// The store's writes in `write_set`, from its own extension bucket.
    fn write_set_ops(write_set: &WriteSet) -> impl Iterator<Item = (&StateKey, &WriteOp)>;

    /// Counts writes applied to the store, labeled `upsert` / `delete`.
    fn writes_counter() -> &'static IntCounterVec;
}

/// Native positions, keyed by `(exchange, account, market)`.
pub(crate) enum PositionStore {}

/// Native orders, keyed by `(exchange, market, order_id)` and
/// `(exchange, market, account)` for bulk orders.
pub(crate) enum OrderStore {}

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

    fn write_set_ops(write_set: &WriteSet) -> impl Iterator<Item = (&StateKey, &WriteOp)> {
        write_set
            .native_position_iter()
            .map(|(key, op)| (key, op.as_write_op()))
    }

    fn writes_counter() -> &'static IntCounterVec {
        &POSITION_WRITES
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

    fn write_set_ops(write_set: &WriteSet) -> impl Iterator<Item = (&StateKey, &WriteOp)> {
        write_set
            .native_order_iter()
            .map(|(key, op)| (key, op.as_write_op()))
    }

    fn writes_counter() -> &'static IntCounterVec {
        &ORDER_WRITES
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
}
